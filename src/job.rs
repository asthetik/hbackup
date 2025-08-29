use crate::{
    file_util::{self, execute_item, execute_item_async},
    item::{get_item, get_items},
    sysexits,
};
use anyhow::Result;
use clap::ValueEnum;
use futures::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, process};
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
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(sysexits::EX_CANTCREAT);
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
    } else {
        let item = get_item(job.clone())?;
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
            eprintln!(
                "The file {target:?} already exists and a directory with the same name cannot be created."
            );
            process::exit(sysexits::EX_CANTCREAT);
        }
        let items = get_items(job.clone())?;
        let mut tasks = FuturesUnordered::new();
        for item in items {
            tasks.push(execute_item_async(item));
        }
        while let Some(res) = tasks.next().await {
            res?;
        }
    } else {
        let item = get_item(job.clone())?;
        execute_item_async(item).await?;
    }
    Ok(())
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
