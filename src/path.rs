use std::error::Error;
use std::fs;
use std::path::PathBuf;

use anyhow::Context;

/// Checks if the given path exists and is accessible.
/// Returns an error if the path is invalid or inaccessible.
pub fn check_path(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    fs::metadata(path).with_context(|| format!("Source path or file '{path:?}' is invalid"))?;
    Ok(())
}

/// Expands a path, replacing `~` with the user's home directory.
pub fn expand_path(path: &str) -> PathBuf {
    let path = shellexpand::tilde(path).into_owned();
    PathBuf::from(path)
}
