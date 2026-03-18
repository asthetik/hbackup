use crate::Result;
use crate::commands::load_config_manager;
use crate::commands::{ProcessCommand, add::CliStrategy};
use clap::Args;
use hbackup_core::engine::executor;
use hbackup_core::model::job::{ArchiveFormat, Job, Level, Strategy};
use std::path::PathBuf;

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

    fn to_temporary_job(&self) -> Job {
        Job {
            id: 0,
            source: self.source.clone().expect("Source path missing"),
            target: self.target.clone().expect("Target path missing"),
            strategy: self.determine_strategy(),
            ignore: self.ignore.clone().unwrap_or_default(),
        }
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
            vec![self.to_temporary_job()]
        };

        if jobs.is_empty() {
            println!("No matching jobs found.");
            return Ok(());
        }

        executor::execute_all(jobs)?;

        Ok(())
    }
}
