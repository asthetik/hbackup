use crate::Result;
use crate::commands::load_config_manager;
use crate::commands::{ProcessCommand, add::CliStrategy};
use clap::Args;
use hbackup_core::engine::executor;
use hbackup_core::error::HbackupError;
use hbackup_core::model::job::{ArchiveFormat, Job, Level, Strategy};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Job id(s) to run.
    #[arg(short, long, required = false, value_delimiter = ',', conflicts_with_all = ["source", "target", "mode", "format", "level"])]
    id: Option<Vec<u32>>,

    /// Source file or directory path (positional, optional). Must be used with target.
    #[arg(required = false, requires = "target")]
    pub source: Option<PathBuf>,

    /// Target file or directory path (positional, optional). Must be used with source.
    #[arg(required = false, requires = "source")]
    pub target: Option<PathBuf>,

    /// Backup mode: mirror, copy, or archive
    #[arg(short, long, required = false)]
    pub mode: Option<CliStrategy>,

    /// Compression format (required only for archive mode)
    #[arg(
        short,
        long,
        value_enum,
        required = false,
        requires_if("mode", "archive")
    )]
    pub format: Option<ArchiveFormat>,

    /// Compression level (required only for archive mode)
    #[arg(short, long, required = false, value_enum, requires = "format")]
    pub level: Option<Level>,

    /// Ignore a specific list of files or directories
    #[arg(short = 'g', long, value_delimiter = ',')]
    ignore: Option<Vec<String>>,
}

impl RunArgs {
    fn is_default_run(&self) -> bool {
        self.id.is_none() && self.source.is_none() && self.target.is_none()
    }

    fn to_temporary_job(&self) -> Result<Job> {
        let source = fs::canonicalize(self.source.clone().expect("Source path missing"))?;
        let target = self.target.clone().expect("Target path missing");

        // Avoid `canonicalize(target)` since target might not exist yet (we want to create it).
        // If it's a relative path, convert to absolute based on current dir.
        let target = if target.is_absolute() {
            target
        } else {
            std::env::current_dir()?.join(target)
        };

        Ok(Job {
            id: 0,
            source,
            target,
            strategy: self.determine_strategy(),
            ignore: self.ignore.clone().unwrap_or_default(),
        })
    }

    fn determine_strategy(&self) -> Strategy {
        match self.mode {
            Some(CliStrategy::Archive) => Strategy::Archive {
                format: self.format.unwrap_or_default(),
                level: self.level.unwrap_or_default(),
            },
            Some(CliStrategy::Mirror) => Strategy::Mirror,
            _ => Strategy::Copy,
        }
    }
}

impl ProcessCommand for RunArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let config = manager.load()?;

        let jobs: Vec<Job> = if self.is_default_run() {
            config.jobs().to_vec()
        } else if let Some(ref ids) = self.id {
            config.list_by_ids(ids).into_iter().cloned().collect()
        } else {
            vec![self.to_temporary_job()?]
        };

        if jobs.is_empty() {
            println!("No matching jobs found.");
            return Ok(());
        }

        let mut set = tokio::task::JoinSet::new();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(3));

        for job in jobs {
            let permit = semaphore.clone().acquire_owned().await?;
            let result_job = job.clone();
            set.spawn(async move {
                // `execute_single` is largely synchronous/blocking (std::fs + compression),
                // so run it in a blocking thread to avoid starving tokio worker threads.
                let join_res =
                    tokio::task::spawn_blocking(move || executor::execute_single(job)).await;

                let result = match join_res {
                    Ok(r) => r,
                    Err(join_err) => Err(HbackupError::RuntimeError(format!(
                        "spawn_blocking join error: {join_err}"
                    ))),
                };
                drop(permit);
                (result_job, result)
            });
        }

        let mut total_success = 0;
        let mut total_fail = 0;

        while let Some(res) = set.join_next().await {
            match res {
                Ok((job, result)) => {
                    let identifier = job.id.to_string();
                    let target = job.target.to_string_lossy();

                    match result {
                        Ok(_) => {
                            total_success += 1;
                            println!("✅ [SUCCESS] Job: {} -> {}", identifier, target);
                        }
                        Err(e) => {
                            total_fail += 1;
                            eprintln!("❌ [FAILED ] Job: {} | Error: {}", identifier, e);
                        }
                    }
                }
                Err(e) => {
                    total_fail += 1;
                    eprintln!("💥 [CRITICAL] A worker thread panicked: {}", e);
                }
            }
        }
        if total_success + total_fail > 1 {
            println!(
                "\n✅ Finished: {} | ❌ Failed: {} | 📦 Total: {}",
                total_success,
                total_fail,
                total_success + total_fail
            );
        }
        Ok(())
    }
}
