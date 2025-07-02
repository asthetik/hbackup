use crate::Result;
use anyhow::Context;
use std::fs;
use std::path::PathBuf;

/// Checks if the given path exists and is accessible.
/// Returns an error if the path is invalid or inaccessible.
pub fn check_path(path: &PathBuf) -> Result<()> {
    fs::metadata(path).with_context(|| format!("The path or file '{path:?}' is invalid"))?;
    Ok(())
}
