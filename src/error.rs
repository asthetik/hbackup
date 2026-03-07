use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HbackupError {
    #[error("path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("compression cannot be used with mirror backup model")]
    InvalidCompressionForMirror,

    #[error("io error: {0}")]
    IoError(#[from] io::Error),

    #[error("maximum number of jobs reached ({0})")]
    TooManyJobs(u32),
}
