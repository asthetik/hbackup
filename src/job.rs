use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a single backup job with a unique id, source, target, and optional compression.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Job {
    /// Unique job id.
    pub id: u32,
    /// Source file or directory path.
    pub source: PathBuf,
    /// Target file or directory path.
    pub target: PathBuf,
    /// Optional compression format for this job.
    pub compression: Option<CompressFormat>,
    /// Optional compression level for this job.
    pub level: Option<Level>,
    /// Optional ignore list
    pub ignore: Option<Vec<String>>,
    /// Backup model
    pub model: Option<BackupModel>,
}

/// Supported compression formats for backup jobs.
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug)]
pub(crate) enum CompressFormat {
    Gzip,
    Zip,
    Sevenz,
    Zstd,
    Bzip2,
    Xz,
    Lz4,
    Tar,
}

/// Supported compression level for backup jobs
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Level {
    Fastest,
    Faster,
    Default,
    Better,
    Best,
}

#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug, Default)]
pub(crate) enum BackupModel {
    #[default]
    Full,
    Mirror,
}
