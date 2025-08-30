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
#[allow(dead_code)]
pub(crate) enum Item {
    Copy { src: PathBuf, dest: PathBuf },
    Ignore { src: PathBuf, dest: PathBuf },
    NotUpdate { src: PathBuf, dest: PathBuf },
    Delete(PathBuf),
}

impl Item {
    fn new_copy(src: &Path, dest: &Path) -> Self {
        Item::Copy {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
        }
    }

    fn new_ignore(src: &Path, dest: &Path) -> Self {
        Item::Ignore {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
        }
    }

    fn new_notupdate(src: &Path, dest: &Path) -> Self {
        Item::NotUpdate {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
        }
    }

    fn new_delete(path: &Path) -> Self {
        Item::Delete(path.to_path_buf())
    }
    fn change_to_delete(&mut self) {
        match self {
            Item::Copy { src: _, dest }
            | Item::Ignore { src: _, dest }
            | Item::NotUpdate { src: _, dest } => {
                *self = Item::Delete(dest.clone());
            }
            Item::Delete(_) => {}
        }
    }
}

pub(crate) fn get_item(job: Job) -> Result<Item> {
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
        BackupModel::Full => Ok(Item::new_copy(&src, &dest)),
        BackupModel::Mirror => {
            if needs_update(&src, &dest)? {
                Ok(Item::new_copy(&src, &dest))
            } else {
                Ok(Item::new_notupdate(&src, &dest))
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
            items.push(Item::new_ignore(entry_path, &dest));
            dest_set.insert(dest);
            continue;
        }
        match model {
            BackupModel::Full => {
                items.push(Item::new_copy(entry_path, &dest));
                dest_set.insert(dest);
            }
            BackupModel::Mirror => {
                if needs_update(entry_path, &dest)? {
                    items.push(Item::new_copy(entry_path, &dest));
                } else {
                    items.push(Item::new_notupdate(entry_path, &dest));
                }
                dest_set.insert(dest);
            }
        }
    }

    for entry in WalkDir::new(&target) {
        let entry = entry?;
        let entry_path = entry.path();
        // Filter entries that match the root target path
        if entry_path == target {
            continue;
        }
        if !dest_set.contains(entry_path) {
            items.push(Item::new_delete(entry_path));
        }
    }
    for item in items.iter_mut() {
        if let Item::Ignore { .. } = item {
            item.change_to_delete();
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
                if dest.is_file() {
                    fs::remove_file(&dest)?;
                } else if dest.is_dir() {
                    fs::remove_dir_all(&dest)?;
                }
            }
        }
        _ => {}
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
                if dest.is_file() {
                    tokio::fs::remove_file(&dest).await?;
                } else if dest.is_dir() {
                    tokio::fs::remove_dir_all(&dest).await?;
                }
            }
        }
        _ => {}
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
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_notupdate(&src, &dest);
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_ignore(&src, &dest);
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

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
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_notupdate(&src, &dest);
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::new_ignore(&src, &dest);
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

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
