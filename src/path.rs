use crate::Result;
use anyhow::Context;
use std::fs;
use std::path::PathBuf;

/// Checks if the given path exists and is accessible.
///
/// # Arguments
/// * `path` - The file or directory path to check.
///
/// # Errors
/// Returns an error if the path does not exist or is not accessible.  
/// The error message includes the problematic path for easier debugging.
///
/// # Example
/// ```
/// use std::path::PathBuf;
/// use hbackup::path::check_path;
/// let path = PathBuf::from("/tmp");
/// let result = check_path(&path);
/// assert!(result.is_ok());
/// ```
pub fn check_path(path: &PathBuf) -> Result<()> {
    fs::metadata(path).with_context(|| format!("The path or file '{path:?}' is invalid"))?;
    Ok(())
}
