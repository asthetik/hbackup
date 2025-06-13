use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};

use anyhow::Context;
use path_clean::PathClean;

/// Checks if the given path exists and is accessible.
/// Returns an error if the path is invalid or inaccessible.
pub fn check_path(path: &PathBuf) -> Result<(), Box<dyn Error>> {
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
