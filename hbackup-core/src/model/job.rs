use crate::error::{HbackupError, Result};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Job {
    pub id: u32,
    pub source: PathBuf,
    pub target: PathBuf,
    pub strategy: Strategy,
    #[serde(default)]
    pub ignore: Vec<String>,
}

impl Job {
    /// Validates the job configuration before it gets added to the config file.
    pub fn validate(&self) -> Result<()> {
        if !self.source.exists() {
            return Err(HbackupError::RuntimeError(format!(
                "Source path does not exist: {:?}",
                self.source
            )));
        }

        if self.source == self.target {
            return Err(HbackupError::RuntimeError(
                "Source and target paths cannot be the same".into(),
            ));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    Mirror,
    Archive { format: ArchiveFormat, level: Level },
    Copy,
}

impl Debug for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Strategy::Mirror => "mirror".to_string(),
            Strategy::Copy => "copy".to_string(),
            Strategy::Archive { format, level } => format!(
                "{{ \"format\": \"{:?}\", \"level\": \"{:?}\" }}",
                format, level
            ),
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Default, Copy)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ArchiveFormat {
    Gzip,
    Zip,
    Sevenz,
    Zstd,
    Bzip2,
    Xz,
    Lz4,
    #[default]
    Tar,
}

impl Debug for ArchiveFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ArchiveFormat::Gzip => "gzip",
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::Sevenz => "7z",
            ArchiveFormat::Zstd => "zstd",
            ArchiveFormat::Bzip2 => "bzip2",
            ArchiveFormat::Xz => "xz",
            ArchiveFormat::Lz4 => "lz4",
            ArchiveFormat::Tar => "tar",
        };
        write!(f, "{}", s)
    }
}

impl ArchiveFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Gzip => "tar.gz",
            Self::Zip => "zip",
            Self::Sevenz => "7z",
            Self::Zstd => "tar.zst",
            Self::Bzip2 => "tar.bz2",
            Self::Xz => "tar.xz",
            Self::Lz4 => "tar.lz4",
            Self::Tar => "tar",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Copy, Default)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Level {
    Fastest,
    Faster,
    #[default]
    Default,
    Better,
    Best,
}

impl Debug for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Level::Fastest => "fastest",
            Level::Faster => "faster",
            Level::Default => "default",
            Level::Better => "better",
            Level::Best => "best",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn validate_source_not_found() {
        let job = Job {
            id: 0,
            source: PathBuf::from("/does/not/exist"),
            target: PathBuf::from("/tmp/target"),
            strategy: Strategy::Copy,
            ignore: vec![],
        };

        let err = job.validate().unwrap_err();
        assert!(matches!(err, crate::error::HbackupError::RuntimeError(_)));
    }

    #[test]
    fn validate_source_equals_target() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("x");
        std::fs::create_dir_all(&path).unwrap();
        let job = Job {
            id: 0,
            source: path.clone(),
            target: path,
            strategy: Strategy::Copy,
            ignore: vec![],
        };
        assert!(matches!(
            job.validate(),
            Err(crate::error::HbackupError::RuntimeError(_))
        ));
    }

    #[test]
    fn validate_ok_path() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();

        let job = Job {
            id: 0,
            source,
            target,
            strategy: Strategy::Copy,
            ignore: vec![],
        };

        assert!(job.validate().is_ok());
    }

    #[test]
    fn validate_source_not_exist() {
        let job = Job {
            id: 0,
            source: PathBuf::from("/does/not/exist"),
            target: PathBuf::from("/tmp/target"),
            strategy: Strategy::Copy,
            ignore: vec![],
        };
        assert!(job.validate().is_err());
    }

    #[test]
    fn validate_source_eq_target() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("x");
        std::fs::create_dir_all(&path).unwrap();
        let job = Job {
            id: 0,
            source: path.clone(),
            target: path,
            strategy: Strategy::Copy,
            ignore: vec![],
        };
        assert!(matches!(
            job.validate(),
            Err(crate::error::HbackupError::RuntimeError(_))
        ));
    }
}
