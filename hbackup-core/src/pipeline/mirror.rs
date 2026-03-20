use crate::error::HbackupError;
use crate::error::Result;
use crate::model::job::Strategy;
use crate::pipeline::stage::Scanner;
use std::fs;
use std::path::PathBuf;
use std::{collections::HashSet, io};
use walkdir::WalkDir;

pub struct SyncExecutor {
    source: PathBuf,
    target: PathBuf,
    ignore_rules: Vec<String>,
}

impl SyncExecutor {
    pub fn new(source: PathBuf, target: PathBuf, ignore_rules: Vec<String>) -> Self {
        Self {
            source,
            target,
            ignore_rules,
        }
    }

    pub fn run(&self, strategy: Strategy) -> Result<()> {
        let scanner = Scanner::new(self.source.clone(), self.ignore_rules.clone());
        let files = scanner.scan()?;
        let keep_files: HashSet<PathBuf> = files.iter().map(|f| f.relative.clone()).collect();

        for file in &files {
            let dest = self.target.join(&file.relative);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&file.absolute, &dest)?;
        }

        if matches!(strategy, Strategy::Mirror) {
            self.cleanup_target_extras(&keep_files)?;
        }

        Ok(())
    }

    /// For `Mirror` mode: delete any file in target that wasn't part of the scanned set.
    ///
    /// Note: we only compare *relative file paths* (not directories), since `Scanner` yields files only.
    fn cleanup_target_extras(&self, keep_files: &HashSet<PathBuf>) -> Result<()> {
        if !self.target.exists() {
            return Ok(());
        }

        let mut dirs: Vec<PathBuf> = Vec::new();

        for entry in WalkDir::new(&self.target).min_depth(1) {
            let entry = entry
                .map_err(|e| HbackupError::RuntimeError(format!("WalkDir entry error: {e}")))?;
            let path = entry.path();

            if entry.file_type().is_dir() {
                dirs.push(path.to_path_buf());
                continue;
            }

            // Treat symlinks as files (delete the link itself) to avoid attempting to traverse.
            if entry.file_type().is_file() || entry.file_type().is_symlink() {
                let relative = path.strip_prefix(&self.target).map_err(|e| {
                    crate::error::HbackupError::RuntimeError(format!("Path alignment error: {e}"))
                })?;

                if !keep_files.contains(relative) {
                    fs::remove_file(path)?;
                }
            }
        }

        // Remove empty directories from bottom to top.
        dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for dir in dirs {
            match fs::remove_dir(&dir) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::job::Strategy;
    use tempfile::tempdir;

    #[test]
    fn mirror_deletes_target_extras() {
        let src = tempdir().unwrap();
        let dst = tempdir().unwrap();

        // Source files we expect to keep.
        let keep_a = src.path().join("a.txt");
        let keep_sub = src.path().join("sub").join("b.txt");
        std::fs::create_dir_all(keep_sub.parent().unwrap()).unwrap();
        std::fs::write(&keep_a, b"hello").unwrap();
        std::fs::write(&keep_sub, b"world").unwrap();

        // Target files we expect to delete.
        let extra_root = dst.path().join("extra.txt");
        let extra_sub = dst.path().join("sub").join("old.txt");
        std::fs::create_dir_all(extra_sub.parent().unwrap()).unwrap();
        std::fs::write(&extra_root, b"unused").unwrap();
        std::fs::write(&extra_sub, b"stale").unwrap();

        let executor =
            SyncExecutor::new(src.path().to_path_buf(), dst.path().to_path_buf(), vec![]);
        executor.run(Strategy::Mirror).unwrap();

        assert!(dst.path().join("a.txt").exists());
        assert!(dst.path().join("sub").join("b.txt").exists());
        assert!(!dst.path().join("extra.txt").exists());
        assert!(!dst.path().join("sub").join("old.txt").exists());
    }

    #[test]
    fn mirror_handles_missing_target_dir() {
        let src = tempdir().unwrap();
        let dst_parent = tempdir().unwrap();
        let missing_target = dst_parent.path().join("does_not_exist");

        std::fs::write(src.path().join("a.txt"), b"hello").unwrap();

        let executor = SyncExecutor::new(src.path().to_path_buf(), missing_target, vec![]);
        executor.run(Strategy::Mirror).unwrap();
    }
}
