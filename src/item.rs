use crate::{file_util, fs};
use crate::{
    file_util::needs_update,
    job::{BackupModel, Job},
};
use anyhow::Context;
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    process,
};
use walkdir::WalkDir;

#[derive(Debug)]
pub(crate) struct Item {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub strategy: Strategy,
}

impl Item {
    fn new(src: PathBuf, dest: PathBuf, strategy: Strategy) -> Item {
        Item {
            src,
            dest,
            strategy,
        }
    }

    pub fn from_delete_strategy(dest: PathBuf) -> Item {
        Self::new(PathBuf::new(), dest, Strategy::Delete)
    }

    pub fn from_copy_strategy(src: PathBuf, dest: PathBuf) -> Item {
        Self::new(src, dest, Strategy::Copy)
    }

    pub fn from_ignore_strategy(src: PathBuf, dest: PathBuf) -> Item {
        Self::new(src, dest, Strategy::Ignore)
    }

    pub fn from_notupdate_strategy(src: PathBuf, dest: PathBuf) -> Item {
        Self::new(src, dest, Strategy::NotUpdate)
    }

    pub fn change_delete_strategy(&mut self) {
        self.src = PathBuf::new();
        self.strategy = Strategy::Delete;
    }
}

#[derive(PartialEq, Debug)]
pub(crate) enum Strategy {
    Copy,
    Ignore,
    NotUpdate,
    Delete,
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
        BackupModel::Full => Ok(Item::new(src, dest, Strategy::Copy)),
        BackupModel::Mirror => {
            if needs_update(&src, &dest)? {
                Ok(Item::new(src, dest, Strategy::Copy))
            } else {
                Ok(Item::new(src, dest, Strategy::NotUpdate))
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
    let mut vec = vec![];
    let ignore_paths: Vec<_> = job
        .ignore
        .as_ref()
        .map(|dirs| dirs.iter().map(|s| src.join(s)).collect())
        .unwrap_or_default();

    for entry in WalkDir::new(&src) {
        let entry = entry?;
        let entry_path = entry.path();
        let rel = entry_path.strip_prefix(prefix)?;
        let dest = target.join(rel);
        if ignore_paths.iter().any(|p| entry_path.starts_with(p)) {
            vec.push(Item::from_ignore_strategy(entry_path.to_path_buf(), dest));
            continue;
        }
        match model {
            BackupModel::Full => {
                vec.push(Item::from_copy_strategy(entry_path.to_path_buf(), dest));
            }
            BackupModel::Mirror => {
                if needs_update(entry_path, &dest)? {
                    vec.push(Item::from_copy_strategy(entry_path.to_path_buf(), dest));
                } else {
                    vec.push(Item::from_notupdate_strategy(
                        entry_path.to_path_buf(),
                        dest,
                    ));
                }
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
        if let Some(i) = vec.iter().position(|v| v.dest.eq(entry_path)) {
            if vec[i].strategy == Strategy::Ignore {
                vec[i].change_delete_strategy();
            }
        } else {
            vec.push(Item::from_delete_strategy(entry_path.to_path_buf()));
        }
    }
    Ok(vec)
}

pub(crate) fn execute_item(item: Item) -> Result<()> {
    let Item {
        src,
        dest,
        strategy,
    } = item;

    match strategy {
        Strategy::Copy => {
            file_util::copy(&src, &dest)?;
        }
        Strategy::Delete => {
            if dest.exists() {
                if dest.is_file() {
                    fs::remove_file(&dest)?;
                } else if dest.is_dir() {
                    fs::remove_dir(&dest)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) async fn execute_item_async(item: Item) -> Result<()> {
    let Item {
        src,
        dest,
        strategy,
    } = item;

    match strategy {
        Strategy::Copy => {
            file_util::copy_async(src, dest).await?;
        }
        Strategy::Delete => {
            if dest.exists() {
                if dest.is_file() {
                    tokio::fs::remove_file(&dest).await?;
                } else if dest.is_dir() {
                    tokio::fs::remove_dir(&dest).await?;
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
        let item = Item::from_copy_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(dest.exists());
        assert!(dest.is_file());
        let output = fs::read_to_string(dest)?;
        assert_eq!(output, "Hello, World!");

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_notupdate_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_ignore_strategy(src.clone(), dest.clone());
        dbg!(&item);
        execute_item(item)?;
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::from_delete_strategy(dest.clone());
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
        let item = Item::from_copy_strategy(src.clone(), dest.clone());
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
        let item = Item::from_notupdate_strategy(src.clone(), dest.clone());
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let src = create_test_file(temp_dir.path(), filename, content);
        let dest = temp_dir.path().join("output").join(filename);
        let item = Item::from_ignore_strategy(src.clone(), dest.clone());
        dbg!(&item);
        rt.block_on(async {
            let res = execute_item_async(item).await;
            assert!(res.is_ok());
        });
        assert!(!dest.exists());

        let temp_dir = TempDir::new()?;
        let dest = create_test_file(temp_dir.path(), filename, content);
        let item = Item::from_delete_strategy(dest.clone());
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
