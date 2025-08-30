use crate::{file_util, fs};
use crate::{
    file_util::needs_update,
    job::{BackupModel, Job},
};
use anyhow::Context;
use anyhow::Result;
use std::collections::HashSet;
use std::{
    path::{Path, PathBuf},
    process,
};
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
        eprintln!("The path {src:?} is not exists");
        process::exit(1);
    } else if !src.is_file() {
        eprintln!("The path {src:?} is not file");
        process::exit(1);
    }

    let dest = job.target;
    let dest = if dest.exists() && dest.is_dir() {
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
        eprintln!("The path {src:?} is not exists");
        process::exit(1);
    } else if !src.is_dir() {
        eprintln!("The path {src:?} is not directory");
        process::exit(1);
    }

    let target = job.target;
    fs::create_dir_all(&target)?;

    let model = job.model.unwrap_or_default();
    // keep previous behavior of including the src dir name in relative path by using parent
    let prefix = src.parent().unwrap_or_else(|| Path::new(""));
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
        let rel = entry_path.strip_prefix(prefix)?;
        let dest = target.join(rel);

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
        // Collect all paths that need to be deleted
        let mut delete_paths = vec![];
        for entry in WalkDir::new(&target) {
            let entry = entry?;
            let entry_path = entry.path();
            // Filter entries that match the root target path
            if entry_path == target {
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

pub(crate) fn execute_item(item: Item) -> Result<()> {
    match item {
        Item::Copy { src, dest } => {
            file_util::copy(&src, &dest)?;
        }
        Item::Delete(dest) => {
            if dest.exists() {
                if dest.is_dir() {
                    if let Err(e) = fs::remove_dir_all(&dest) {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            eprintln!("Failed to delete directory {dest:?}: {e}");
                        }
                    }
                } else if let Err(e) = fs::remove_file(&dest) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        eprintln!("Failed to delete file {dest:?}: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) async fn execute_item_async(item: Item) -> Result<()> {
    match item {
        Item::Copy { src, dest } => {
            file_util::copy_async(src, dest).await?;
        }
        Item::Delete(dest) => {
            if dest.exists() {
                if dest.is_dir() {
                    if let Err(e) = tokio::fs::remove_dir_all(&dest).await {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            eprintln!("Failed to delete directory {dest:?}: {e}");
                        }
                    }
                } else if let Err(e) = tokio::fs::remove_file(&dest).await {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        eprintln!("Failed to delete file {dest:?}: {e}");
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_execute_item() -> Result<()> {
        let filename = "hello.txt";
        let content = b"Hello, World!";

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_copy(&src, &dest);
        dbg!(&item);
        execute_item(item)?;
        assert!(dest.exists());
        assert!(dest.is_file());
        let output = fs::read_to_string(dest)?;
        assert_eq!(output, "Hello, World!");

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::new_delete(&dest);
        dbg!(&item);
        assert!(dest.exists());
        execute_item(item)?;
        assert!(!dest.exists());

        Ok(())
    }

    #[test]
    fn test_execute_item_async() -> Result<()> {
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
            let res = execute_item_async(item).await;
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
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        Ok(())
    }
}
