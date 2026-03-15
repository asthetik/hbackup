use thiserror::Error;

pub type Result<T> = std::result::Result<T, HbackupError>;

#[derive(Error, Debug)]
pub enum HbackupError {
    /// Wrapper for standard IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Wrapper for TOML deserialization errors (loading config)
    #[error("Failed to parse config file: {0}")]
    ConfigDeserialize(#[from] toml::de::Error),

    /// Wrapper for TOML serialization errors (saving config)
    #[error("Failed to serialize config: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    /// Used when system environment is unavailable (e.g., no home dir)
    #[error("Environment unavailable: {0}")]
    EnvironmentUnavailable(String),

    /// Logic error when a job ID does not exist
    #[error("Job with ID {0} not found")]
    JobNotFound(u32),

    /// Logic error when too many jobs are added
    #[error("Maximum job limit reached: {0}")]
    TooManyJobs(u32),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// General runtime errors
    #[error("Operation failed: {0}")]
    RuntimeError(String),
}