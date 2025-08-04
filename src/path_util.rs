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
pub fn check_path(path: &PathBuf) -> Result<()> {
    fs::metadata(path).with_context(|| format!("The path or file '{path:?}' is invalid"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_check_path_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        fs::write(&file_path, "test content").unwrap();

        let result = check_path(&file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_path_valid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("test_directory");
        fs::create_dir(&dir_path).unwrap();

        let result = check_path(&dir_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_path_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent_file.txt");

        let result = check_path(&nonexistent_path);
        assert!(result.is_err());

        let error_msg = format!("{}", result.unwrap_err());
        assert!(error_msg.contains("invalid"));
        assert!(error_msg.contains("nonexistent_file.txt"));
    }

    #[test]
    fn test_check_path_nested_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("level1").join("level2").join("level3");
        fs::create_dir_all(&nested_dir).unwrap();

        let result = check_path(&nested_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_path_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "").unwrap();

        let result = check_path(&empty_file);
        assert!(result.is_ok());
    }
}
