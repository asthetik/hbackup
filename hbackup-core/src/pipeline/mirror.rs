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

    /// Executes the backup operation using the specified strategy.
    ///
    /// Scans the source directory for files (respecting any configured ignore rules),
    /// then copies each file to the target directory, preserving the original
    /// relative path structure. Target parent directories are created automatically.
    ///
    /// # Strategies
    ///
    /// - [`Strategy::Copy`]   — Copy files to target; any pre-existing files in
    ///   target that are **not** part of the scanned set are left untouched.
    /// - [`Strategy::Mirror`] — Copy files to target, then **delete** every file
    ///   and empty directory in target that is not present in the scanned set,
    ///   making target an exact mirror of source.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if:
    /// - The scanner fails to enumerate source files.
    /// - A target parent directory cannot be created.
    /// - A file copy operation fails.
    /// - *(Mirror only)* Cleaning up extra target entries fails.
    pub fn run(&self, strategy: Strategy) -> Result<()> {
        let scanner = Scanner::new(self.source.clone(), self.ignore_rules.clone());
        let files = scanner.scan()?;

        // Build the set of relative file paths that must be kept.
        let keep_files: HashSet<PathBuf> = files.iter().map(|f| f.relative.clone()).collect();

        // Build the set of all ancestor directories for kept files so that
        // cleanup can skip removal attempts on directories known to contain
        // retained content.
        //
        // Short-circuit: once a directory is already present in the set, all of
        // its ancestors must have been inserted during an earlier iteration, so
        // we can break immediately. This avoids redundant work and eliminates the
        // per-file Vec allocation that a flat_map approach would require.
        //
        // Note: the root ("") is intentionally not inserted — it is excluded from
        // the WalkDir pass via min_depth(1) and therefore never needs special-casing.
        let mut keep_dirs: HashSet<PathBuf> = HashSet::new();
        for file in &files {
            let mut cur = file.relative.parent();
            while let Some(dir) = cur {
                if !keep_dirs.insert(dir.to_path_buf()) {
                    break; // dir already present → all ancestors already inserted
                }
                cur = dir.parent();
            }
        }

        // Copy every scanned file to target, creating parent directories as needed.
        // `created_dirs` avoids redundant `create_dir_all` syscalls when several
        // files share the same parent.
        let mut created_dirs: HashSet<PathBuf> = HashSet::new();
        for file in &files {
            let dest = self.target.join(&file.relative);
            if let Some(parent) = dest.parent()
                && created_dirs.insert(parent.to_path_buf())
            {
                fs::create_dir_all(parent)?;
            }

            fs::copy(&file.absolute, &dest)?;
        }

        if matches!(strategy, Strategy::Mirror) {
            self.cleanup_target_extras(&keep_files, &keep_dirs)?;
        }

        Ok(())
    }

    /// Removes files and empty directories from the target that are not part of
    /// the scanned (keep) set. Called only in [`Strategy::Mirror`] mode.
    ///
    /// # Algorithm
    ///
    /// 1. Walks the entire target tree (depth ≥ 1) with [`WalkDir`] in
    ///    **contents-first** order, without following symbolic links.
    ///    Contents-first guarantees that every entry inside a directory is
    ///    yielded *before* the directory itself, so a single pass handles both
    ///    file deletion and directory cleanup without any sorting step.
    /// 2. **Files / symlinks** — deleted immediately if their target-relative
    ///    path is not in `keep_files`.  Symlinks are treated as plain files: the
    ///    link itself is removed rather than its referent, which prevents
    ///    accidental traversal into foreign directory trees.
    /// 3. **Directories** — attempted for removal after all their contents have
    ///    already been processed.  Any directory whose target-relative path
    ///    appears in `keep_dirs` is skipped unconditionally.  Empty directories
    ///    are removed; [`io::ErrorKind::DirectoryNotEmpty`] and
    ///    [`io::ErrorKind::NotFound`] are silently ignored (the former means the
    ///    directory still holds kept content; the latter means a prior removal
    ///    already cleaned it up).
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if any file removal fails, or if a directory removal
    /// returns an error other than `DirectoryNotEmpty` or `NotFound`.
    fn cleanup_target_extras(
        &self,
        keep_files: &HashSet<PathBuf>,
        keep_dirs: &HashSet<PathBuf>,
    ) -> Result<()> {
        if !self.target.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&self.target)
            .min_depth(1)
            .follow_links(false) // explicit: never follow symlinks into foreign trees
            .contents_first(true)
        // children before parents → no sort needed
        {
            let entry = entry
                .map_err(|e| HbackupError::RuntimeError(format!("WalkDir entry error: {e}")))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(&self.target)
                .map_err(|e| HbackupError::RuntimeError(format!("Path alignment error: {e}")))?;

            if entry.file_type().is_dir() {
                // Skip directories that are ancestors of kept files — they must stay.
                if keep_dirs.contains(relative) {
                    continue;
                }
                match fs::remove_dir(path) {
                    Ok(_) => {}
                    // Non-empty: still holds kept content — leave it alone.
                    Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {}
                    // Already gone (e.g. removed as part of a prior subtree).
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            } else if entry.file_type().is_file() || entry.file_type().is_symlink() {
                // Treat symlinks as files: remove the link itself, not its referent.
                if !keep_files.contains(relative) {
                    fs::remove_file(path)?;
                }
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
