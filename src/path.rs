use crate::Result;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

/// Checks if the given path exists and is accessible.
/// Returns an error if the path is invalid or inaccessible.
pub(crate) fn check_path(path: &PathBuf) -> Result<()> {
    fs::metadata(path).with_context(|| format!("The path or file '{path:?}' is invalid"))?;
    Ok(())
}

/// get all files
pub(crate) fn get_all_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let mut sub_files = get_all_files(&path)?;
            result.append(&mut sub_files);
        } else {
            result.push(path);
        }
    }

    Ok(result)
}
