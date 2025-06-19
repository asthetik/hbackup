use crate::Result;
use anyhow::Context;
use path_clean::PathClean;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Checks if the given path exists and is accessible.
/// Returns an error if the path is invalid or inaccessible.
pub fn check_path(path: &PathBuf) -> Result<()> {
    fs::metadata(path).with_context(|| format!("Source path or file '{path:?}' is invalid"))?;
    Ok(())
}

/// Expands a path, replacing `~` with the user's home directory.
pub fn expand_path(path: &str) -> PathBuf {
    let path = expand_home(path);
    let path = Path::new(&path);

    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .expect("Unable to find current path")
            .join(path)
    };
    abs_path.clean()
}

/// expand home
fn expand_home(input: &str) -> String {
    if input.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            return input.replacen("~", &home.to_string_lossy(), 1);
        }
    } else if input.starts_with("$HOME") {
        if let Some(home) = dirs::home_dir() {
            return input.replacen("$HOME", &home.to_string_lossy(), 1);
        }
    }
    input.into()
}

/// get all files
pub fn get_all_files(path: &Path) -> Result<Vec<PathBuf>> {
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
