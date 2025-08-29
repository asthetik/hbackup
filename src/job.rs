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

pub(crate) fn display_jobs(jobs: Vec<Job>) -> String {
    if jobs.is_empty() {
        return String::new();
    }
    let mut s = String::from('[');
    for job in jobs {
        let comp = match job.compression {
            Some(CompressFormat::Gzip) => "Gzip",
            Some(CompressFormat::Zip) => "Zip",
            Some(CompressFormat::Sevenz) => "Sevenz",
            Some(CompressFormat::Zstd) => "Zstd",
            Some(CompressFormat::Bzip2) => "Bzip2",
            Some(CompressFormat::Xz) => "Xz",
            Some(CompressFormat::Lz4) => "Lz4",
            Some(CompressFormat::Tar) => "Tar",
            None => "",
        };
        let level = match job.level {
            Some(Level::Fastest) => "Fastest",
            Some(Level::Faster) => "Faster",
            Some(Level::Default) => "Default",
            Some(Level::Better) => "Better",
            Some(Level::Best) => "Best",
            None => "",
        };
        let model = match job.model {
            Some(BackupModel::Full) => "Full",
            Some(BackupModel::Mirror) => "Mirror",
            None => "",
        };
        s.push_str(&format!(
            "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\"",
            job.id,
            job.source.display(),
            job.target.display()
        ));
        if !comp.is_empty() {
            s.push_str(&format!(",\n    compression: \"{comp}\""));
        }
        if !level.is_empty() {
            s.push_str(&format!(",\n    level: \"{level}\""));
        }
        if let Some(ignore) = &job.ignore {
            s.push_str(&format!(",\n    ignore: {ignore:?}"));
        }
        if !model.is_empty() {
            s.push_str(&format!(",\n    model: \"{model}\""));
        }
        s.push_str("\n}");
    }
    s.push(']');
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_job_list_display() {
        let jobs = vec![
            Job {
                id: 1,
                source: PathBuf::from("/test/source1"),
                target: PathBuf::from("/test/target1"),
                compression: Some(CompressFormat::Zip),
                level: Some(Level::Fastest),
                ignore: None,
                model: None,
            },
            Job {
                id: 2,
                source: PathBuf::from("/test/source2"),
                target: PathBuf::from("/test/target2"),
                compression: Some(CompressFormat::Zstd),
                level: Some(Level::Best),
                ignore: Some(vec!["*.tmp".to_string()]),
                model: None,
            },
        ];

        let display_str = display_jobs(jobs);

        assert!(display_str.starts_with('['));
        assert!(display_str.ends_with(']'));
        assert!(display_str.contains("id: 1"));
        assert!(display_str.contains("id: 2"));
        assert!(display_str.contains("Zip"));
        assert!(display_str.contains("Zstd"));
    }
}
