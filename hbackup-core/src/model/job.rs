use crate::error::{HbackupError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub id: u32,
    pub source: PathBuf,
    pub target: PathBuf,
    pub strategy: Strategy,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum Strategy {
    Mirror,
    Archive { format: ArchiveFormat, level: Level },
    Copy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum ArchiveFormat {
    Gzip,
    Zip,
    Sevenz,
    Zstd,
    Bzip2,
    Xz,
    Lz4,
    Tar,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum Level {
    Fastest,
    Faster,
    Default,
    Better,
    Best,
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
        };
        assert!(matches!(
            job.validate(),
            Err(crate::error::HbackupError::RuntimeError(_))
        ));
    }
}
