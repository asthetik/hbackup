use std::error::Error;
use std::fs;
use std::path::PathBuf;

pub fn check_path(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    fs::metadata(path).map_err(|e| format!("Source path or file '{path:?}' is invalid: {e}"))?;
    Ok(())
}

pub fn expand_path(path: &str) -> PathBuf {
    let path = shellexpand::tilde(path).into_owned();
    PathBuf::from(path)
}
