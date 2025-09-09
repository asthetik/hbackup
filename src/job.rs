use crate::{
    file_util,
    item::{execute_item, execute_item_async, get_item, get_items},
};
use anyhow::Result;
use anyhow::anyhow;
use clap::ValueEnum;
use futures::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::runtime::Builder as runtimeBuilder;

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
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug, PartialEq)]
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
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) enum Level {
    Fastest,
    Faster,
    Default,
    Better,
    Best,
}

#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub(crate) enum BackupModel {
    #[default]
    Full,
    Mirror,
}

impl Job {
    pub(crate) fn temp_job(
        source: PathBuf,
        target: PathBuf,
        compression: Option<CompressFormat>,
        level: Option<Level>,
        ignore: Option<Vec<String>>,
        model: Option<BackupModel>,
    ) -> Job {
        Job {
            id: 0,
            source,
            target,
            compression,
            level,
            ignore,
            model,
        }
    }
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

/// Runs a backup job (single file or directory copy, with optional compression).
pub(crate) fn run_job(job: &Job) -> Result<()> {
    if let Some(ref format) = job.compression {
        let level = job.level.as_ref().unwrap_or(&Level::Default);
        file_util::compression(&job.source, &job.target, format, level, &job.ignore)?;
    } else if job.source.is_dir() {
        let target = &job.target;
        if target.exists() && target.is_file() {
            return Err(anyhow!(
                "The file {target:?} already exists and a directory with the same name cannot be created."
            ));
        }

        let items = get_items(job.clone())?;
        let rt = runtimeBuilder::new_multi_thread().enable_all().build()?;
        rt.block_on(async {
            let mut tasks = FuturesUnordered::new();
            for item in items {
                tasks.push(execute_item_async(item));
            }
            while let Some(res) = tasks.next().await {
                res?;
            }
            Ok::<(), anyhow::Error>(())
        })?;
    } else if let Some(item) = get_item(job.clone())? {
        execute_item(item)?;
    }
    Ok(())
}

/// Runs multiple backup jobs concurrently.
pub(crate) fn run_jobs(jobs: Vec<Job>) -> Result<()> {
    let rt = runtimeBuilder::new_multi_thread().enable_all().build()?;

    rt.block_on(async move {
        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            set.spawn(async move {
                if let Err(e) = run_job_async(&job).await {
                    eprintln!("Failed to run job with id {}: {}\n", job.id, e);
                }
            });
        }
        while let Some(res) = set.join_next().await {
            if let Err(e) = res {
                eprintln!("Failed to run job: {e}\n");
            }
        }
    });

    Ok(())
}

/// Runs a backup job (single file or directory copy, with optional compression).
async fn run_job_async(job: &Job) -> Result<()> {
    if let Some(ref format) = job.compression {
        let level = job.level.as_ref().unwrap_or(&Level::Default);
        let src = job.source.clone();
        let tgt = job.target.clone();
        let fmt = format.clone();
        let lvl = level.clone();
        let ignore = job.ignore.clone();
        tokio::task::spawn_blocking(move || {
            file_util::compression(&src, &tgt, &fmt, &lvl, &ignore)
        })
        .await??;
    } else if job.source.is_dir() {
        let target = &job.target;
        if target.exists() && target.is_file() {
            return Err(anyhow!(
                "The file {target:?} already exists and a directory with the same name cannot be created."
            ));
        }
        let items = get_items(job.clone())?;
        let mut tasks = FuturesUnordered::new();
        for item in items {
            tasks.push(execute_item_async(item));
        }
        while let Some(res) = tasks.next().await {
            res?;
        }
    } else if let Some(item) = get_item(job.clone())? {
        execute_item_async(item).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_dir(name: &str) -> PathBuf {
        TempDir::new().unwrap().path().join(name)
    }

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

    #[test]
    fn test_empty_job_list_display() {
        let jobs = vec![];
        let display_str = display_jobs(jobs);
        assert_eq!(display_str, "");
    }

    #[test]
    fn test_job_display_with_all_compression_formats() {
        let formats = [
            CompressFormat::Gzip,
            CompressFormat::Zip,
            CompressFormat::Sevenz,
            CompressFormat::Zstd,
            CompressFormat::Bzip2,
            CompressFormat::Xz,
            CompressFormat::Lz4,
            CompressFormat::Tar,
        ];

        for (i, format) in formats.iter().enumerate() {
            let source = create_test_dir("input");
            let target = TempDir::new().unwrap().path().join("output");
            let job = Job {
                id: (i + 1) as u32,
                source,
                target,
                compression: Some(format.clone()),
                level: Some(Level::Default),
                ignore: None,
                model: None,
            };

            let display_str = display_jobs(vec![job]);
            assert!(display_str.contains(&format!("{:?}", format)));
        }
    }

    #[test]
    fn test_job_display_with_all_compression_levels() {
        let levels = [
            Level::Fastest,
            Level::Faster,
            Level::Default,
            Level::Better,
            Level::Best,
        ];

        for (i, level) in levels.iter().enumerate() {
            let source = create_test_dir("input");
            let target = TempDir::new().unwrap().path().join("output");
            let job = Job {
                id: (i + 1) as u32,
                source,
                target,
                compression: Some(CompressFormat::Gzip),
                level: Some(level.clone()),
                ignore: None,
                model: None,
            };

            let display_str = display_jobs(vec![job]);
            assert!(display_str.contains(&format!("{:?}", level)));
        }
    }

    #[test]
    fn test_job_display_with_backup_models() {
        let models = [BackupModel::Full, BackupModel::Mirror];

        for (i, model) in models.iter().enumerate() {
            let source = create_test_dir("input");
            let target = TempDir::new().unwrap().path().join("output");
            let job = Job {
                id: (i + 1) as u32,
                source,
                target,
                compression: None,
                level: None,
                ignore: None,
                model: Some(model.clone()),
            };

            let display_str = display_jobs(vec![job]);
            assert!(display_str.contains(&format!("{:?}", model)));
        }
    }

    #[test]
    fn test_job_display_without_optional_fields() {
        let source = create_test_dir("input");
        let target = TempDir::new().unwrap().path().join("output");
        let job = Job {
            id: 1,
            source: source.clone(),
            target: target.clone(),
            compression: None,
            level: None,
            ignore: None,
            model: None,
        };

        let display_str = display_jobs(vec![job]);

        // Should contain required fields
        assert!(display_str.contains("id: 1"));
        assert!(display_str.contains(&format!("source: {:?}", source)));
        assert!(display_str.contains(&format!("target: {:?}", target)));

        // Should not contain optional fields when they're None
        assert!(!display_str.contains("compression:"));
        assert!(!display_str.contains("level:"));
        assert!(!display_str.contains("ignore:"));
        assert!(!display_str.contains("model:"));
    }

    #[test]
    fn test_job_display_with_ignore_patterns() {
        let source = create_test_dir("input");
        let target = TempDir::new().unwrap().path().join("output");
        let job = Job {
            id: 1,
            source,
            target,
            compression: None,
            level: None,
            ignore: Some(vec![
                "*.log".to_string(),
                "*.tmp".to_string(),
                "cache/".to_string(),
            ]),
            model: None,
        };

        let display_str = display_jobs(vec![job]);

        assert!(display_str.contains("ignore:"));
        assert!(display_str.contains("*.log"));
        assert!(display_str.contains("*.tmp"));
        assert!(display_str.contains("cache/"));
    }

    #[test]
    fn test_temp_job_creation() {
        let source = create_test_dir("input");
        let target = TempDir::new().unwrap().path().join("output");
        let compression = Some(CompressFormat::Gzip);
        let level = Some(Level::Best);
        let ignore = Some(vec!["*.log".to_string()]);
        let model = Some(BackupModel::Mirror);

        let job = Job::temp_job(
            source.clone(),
            target.clone(),
            compression.clone(),
            level.clone(),
            ignore.clone(),
            model.clone(),
        );

        assert_eq!(job.id, 0);
        assert_eq!(job.source, source);
        assert_eq!(job.target, target);
        assert_eq!(job.compression, compression);
        assert_eq!(job.level, level);
        assert_eq!(job.ignore, ignore);
        assert_eq!(job.model, model);
    }

    #[test]
    fn test_backup_model_default() {
        let model = BackupModel::default();
        assert_eq!(model, BackupModel::Full);
    }

    #[test]
    fn test_job_serialization() {
        let source = create_test_dir("input");
        let target = TempDir::new().unwrap().path().join("output");
        let job = Job {
            id: 42,
            source,
            target,
            compression: Some(CompressFormat::Zstd),
            level: Some(Level::Better),
            ignore: Some(vec!["*.tmp".to_string(), ".DS_Store".to_string()]),
            model: Some(BackupModel::Mirror),
        };

        // Test serialization to TOML
        let toml_str = toml::to_string(&job).expect("Failed to serialize job to TOML");
        assert!(toml_str.contains("id = 42"));
        assert!(toml_str.contains("Zstd"));
        assert!(toml_str.contains("Better"));
        assert!(toml_str.contains("Mirror"));

        // Test deserialization from TOML
        let deserialized: Job =
            toml::from_str(&toml_str).expect("Failed to deserialize job from TOML");
        assert_eq!(deserialized.id, job.id);
        assert_eq!(deserialized.source, job.source);
        assert_eq!(deserialized.target, job.target);
        assert_eq!(deserialized.compression, job.compression);
        assert_eq!(deserialized.level, job.level);
        assert_eq!(deserialized.ignore, job.ignore);
        assert_eq!(deserialized.model, job.model);
    }

    #[test]
    fn test_multiple_jobs_display_formatting() {
        let jobs = vec![
            Job {
                id: 1,
                source: create_test_dir("/path1"),
                target: create_test_dir("/target1"),
                compression: Some(CompressFormat::Gzip),
                level: Some(Level::Fastest),
                ignore: None,
                model: Some(BackupModel::Full),
            },
            Job {
                id: 2,
                source: create_test_dir("/path2"),
                target: create_test_dir("/target2"),
                compression: None,
                level: None,
                ignore: Some(vec!["*.log".to_string()]),
                model: Some(BackupModel::Mirror),
            },
        ];

        let display_str = display_jobs(jobs);

        // Should start with [ and end with ]
        assert!(display_str.starts_with('['));
        assert!(display_str.ends_with(']'));

        // Should contain both jobs
        assert!(display_str.contains("id: 1"));
        assert!(display_str.contains("id: 2"));

        // Should have proper structure with braces
        let open_braces = display_str.matches('{').count();
        let close_braces = display_str.matches('}').count();
        assert_eq!(open_braces, close_braces);
        assert_eq!(open_braces, 2); // One for each job
    }
}
