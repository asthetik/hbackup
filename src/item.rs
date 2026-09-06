use crate::error::HbackupError;
use crate::file_util;
use crate::job::{BackupModel, Job};
use anyhow::Context;
use anyhow::Result;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio::sync::Semaphore;
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) enum Item {
    Copy { src: PathBuf, dest: PathBuf },
    Delete(PathBuf),
}

impl Item {
    fn new_copy(src: &Path, dest: &Path) -> Self {
        Item::Copy {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
        }
    }

    fn new_delete(path: &Path) -> Self {
        Item::Delete(path.to_path_buf())
    }
}

pub(crate) fn get_item(job: Job) -> Result<Option<Item>> {
    let src = job.source;
    if !src.exists() {
        return Err(HbackupError::PathNotFound(src).into());
    } else if !src.is_file() {
        return Err(HbackupError::NotAFile(src).into());
    }

    let dest = job.target;
    // A target that does not exist yet is treated as a directory to create
    // (the backup output keeps its file name inside it); an existing file
    // target is used as the exact destination path.
    let dest = if !dest.exists() || dest.is_dir() {
        let file_name = src.file_name().with_context(|| "Invalid file name")?;
        dest.join(file_name)
    } else {
        dest
    };
    let model = job.model.unwrap_or_default();
    match model {
        BackupModel::Full => Ok(Some(Item::new_copy(&src, &dest))),
        BackupModel::Mirror => {
            if needs_update(&src, &dest)? {
                Ok(Some(Item::new_copy(&src, &dest)))
            } else {
                Ok(None)
            }
        }
    }
}

pub(crate) fn get_items(job: Job) -> Result<Vec<Item>> {
    let src = job.source;
    if !src.exists() {
        return Err(HbackupError::PathNotFound(src).into());
    } else if !src.is_dir() {
        return Err(HbackupError::NotADirectory(src).into());
    }

    let model = job.model.unwrap_or_default();
    let src_name = src.file_name().with_context(|| "Invalid file name")?;
    let dest = job.target.join(src_name);

    // keep previous behavior of including the src dir name in relative path by using parent
    let mut items = vec![];
    let ignore_paths: Vec<_> = job
        .ignore
        .as_ref()
        .map(|dirs| dirs.iter().map(|s| src.join(s)).collect())
        .unwrap_or_default();

    let mut dest_set = HashSet::new();

    for entry in WalkDir::new(&src) {
        let entry = entry?;
        let entry_path = entry.path();
        let rel = entry_path.strip_prefix(&src)?;
        let dest = dest.join(rel);
        if ignore_paths.iter().any(|p| entry_path.starts_with(p)) {
            continue;
        }
        match model {
            BackupModel::Full => {
                items.push(Item::new_copy(entry_path, &dest));
            }
            BackupModel::Mirror => {
                if needs_update(entry_path, &dest)? {
                    items.push(Item::new_copy(entry_path, &dest));
                }
                dest_set.insert(dest);
            }
        }
    }

    if let BackupModel::Mirror = model {
        if !dest.exists() {
            return Ok(items);
        }
        // Collect all paths that need to be deleted
        let mut delete_paths = vec![];
        for entry in WalkDir::new(&dest) {
            let entry = entry?;
            let entry_path = entry.path();
            // Filter entries that match the root dest path
            if entry_path == dest {
                continue;
            }
            if !dest_set.contains(entry_path) {
                delete_paths.push(entry_path.to_path_buf());
            }
        }
        if delete_paths.is_empty() {
            return Ok(items);
        }

        // Sort by path length, prioritizing top-level directories
        delete_paths.sort_by_key(|p| p.components().count());
        // Only keep items that are not included in other to-be-deleted paths
        let mut filtered = vec![];
        for path in delete_paths {
            if !filtered.iter().any(|parent| path.starts_with(parent)) {
                filtered.push(path);
            }
        }
        for path in filtered {
            items.push(Item::new_delete(&path));
        }
    }
    Ok(items)
}

pub(crate) async fn execute_item_async(item: Item, permits: Arc<Semaphore>) -> Result<()> {
    // One permit per in-flight item keeps the open files of all concurrent
    // jobs within low ulimits; it is dropped when the item finishes.
    let _permit = permits
        .acquire()
        .await
        .context("failed to acquire copy permit")?;
    match item {
        Item::Copy { src, dest } => {
            file_util::copy_async(src, dest).await?;
        }
        Item::Delete(dest) => {
            if dest.exists() {
                if dest.is_dir() {
                    if let Err(e) = tokio::fs::remove_dir_all(&dest).await
                        && e.kind() != std::io::ErrorKind::NotFound
                    {
                        eprintln!("Failed to delete directory {dest:?}: {e}");
                    }
                } else if let Err(e) = tokio::fs::remove_file(&dest).await
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!("Failed to delete file {dest:?}: {e}");
                }
            }
        }
    }
    Ok(())
}

fn needs_update(src: &Path, dest: &Path) -> Result<bool> {
    if !dest.exists() {
        return Ok(true);
    }

    let sm = fs::metadata(src).context(format!(
        "Failed to get metadata for source file: {}",
        src.display()
    ))?;
    let dm = fs::metadata(dest).context(format!(
        "Failed to get metadata for destination file: {}",
        dest.display()
    ))?;
    if sm.len() != dm.len() {
        return Ok(true);
    }

    let s_mod = sm.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let d_mod = dm.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    const TOLERANCE: Duration = Duration::from_secs(1);
    if s_mod > d_mod + TOLERANCE {
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::COPY_CONCURRENCY;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    #[test]
    fn test_execute_item_async() -> Result<()> {
        let _serial = file_util::test_hooks::copy_test_lock();
        let filename = "hello.txt";
        let content = b"Hello, World!";
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_copy(&src, &dest);
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item, Arc::new(Semaphore::new(COPY_CONCURRENCY))).await;
            assert!(res.is_ok());
        });
        assert!(dest.exists());
        assert!(dest.is_file());
        let output = fs::read_to_string(dest)?;
        assert_eq!(output, "Hello, World!");

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::new_delete(&dest);
        dbg!(&item);
        assert!(dest.exists());
        rt.block_on(async {
            let res = execute_item_async(item, Arc::new(Semaphore::new(COPY_CONCURRENCY))).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        Ok(())
    }

    #[test]
    fn test_get_item_nonexistent_target_treated_as_dir() -> Result<()> {
        let temp = TempDir::new()?;
        let src = create_test_file(temp.path(), "a.txt", b"data");
        let target = temp.path().join("new_dir");
        let job = Job::temp_job(src, target, None, None, None, None);
        let item = get_item(job)?.expect("copy item for existing source");
        match item {
            Item::Copy { dest, .. } => {
                assert_eq!(dest, temp.path().join("new_dir").join("a.txt"))
            }
            Item::Delete(_) => panic!("expected a copy item"),
        }
        Ok(())
    }

    #[test]
    fn test_get_item_source_not_found_is_err() {
        let temp = TempDir::new().unwrap();
        let job = Job::temp_job(
            temp.path().join("missing.txt"),
            temp.path().join("out"),
            None,
            None,
            None,
            None,
        );
        let err = get_item(job).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<HbackupError>(),
            Some(HbackupError::PathNotFound(_))
        ));
    }

    #[test]
    fn test_get_item_source_is_dir_is_err() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("dir");
        fs::create_dir_all(&src).unwrap();
        let job = Job::temp_job(src, temp.path().join("out"), None, None, None, None);
        let err = get_item(job).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<HbackupError>(),
            Some(HbackupError::NotAFile(_))
        ));
    }

    #[test]
    fn test_get_items_source_not_found_is_err() {
        let temp = TempDir::new().unwrap();
        let job = Job::temp_job(
            temp.path().join("missing_dir"),
            temp.path().join("out"),
            None,
            None,
            None,
            None,
        );
        let err = get_items(job).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<HbackupError>(),
            Some(HbackupError::PathNotFound(_))
        ));
    }

    #[test]
    fn test_get_items_source_is_file_is_err() {
        let temp = TempDir::new().unwrap();
        let src = create_test_file(temp.path(), "file.txt", b"data");
        let job = Job::temp_job(src, temp.path().join("out"), None, None, None, None);
        let err = get_items(job).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<HbackupError>(),
            Some(HbackupError::NotADirectory(_))
        ));
    }

    #[test]
    fn test_needs_update_missing_dest_copies() -> Result<()> {
        let temp = TempDir::new()?;
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir)?;
        let src = create_test_file(&src_dir, "a.txt", b"v1");
        let dest = temp.path().join("a.txt");
        assert!(needs_update(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn test_needs_update_unchanged_skips() -> Result<()> {
        use filetime::{FileTime, set_file_mtime};

        let temp = TempDir::new()?;
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir)?;
        let src = create_test_file(&src_dir, "a.txt", b"v1");
        let dest = temp.path().join("a.txt");
        fs::copy(&src, &dest)?;
        // Pin mtimes: dest "copied" 100s after src, same size -> skip.
        set_file_mtime(&src, FileTime::from_unix_time(1_700_000_000, 0))?;
        set_file_mtime(&dest, FileTime::from_unix_time(1_700_000_100, 0))?;
        assert!(!needs_update(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn test_needs_update_size_changed_copies() -> Result<()> {
        let temp = TempDir::new()?;
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir)?;
        let src = create_test_file(&src_dir, "a.txt", b"v1");
        let dest = temp.path().join("a.txt");
        fs::copy(&src, &dest)?;
        // Size check runs before mtime, so no timing tricks needed.
        create_test_file(&src_dir, "a.txt", b"longer content");
        assert!(needs_update(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn test_needs_update_mtime_changed_copies() -> Result<()> {
        use filetime::{FileTime, set_file_mtime};

        let temp = TempDir::new()?;
        let src_dir = temp.path().join("src");
        fs::create_dir_all(&src_dir)?;
        let src = create_test_file(&src_dir, "a.txt", b"v1");
        let dest = temp.path().join("a.txt");
        fs::copy(&src, &dest)?;
        // Same size, but src mtime pinned 35 minutes after dest: past the
        // 1s tolerance, no sleep needed on any filesystem.
        set_file_mtime(&dest, FileTime::from_unix_time(1_700_000_000, 0))?;
        set_file_mtime(&src, FileTime::from_unix_time(1_700_002_100, 0))?;
        assert!(needs_update(&src, &dest)?);
        Ok(())
    }
}
