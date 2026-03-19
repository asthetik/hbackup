use crate::Result;
use crate::commands::{ProcessCommand, load_config_manager};
use clap::Args;
use hbackup_core::model::job::{ArchiveFormat, Job, Level, Strategy};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Source directory or file to back up
    pub source: PathBuf,

    /// Target directory where the backup will be stored
    pub target: PathBuf,

    /// Backup mode: mirror, copy, or archive
    #[arg(short, long, value_enum, default_value_t = CliStrategy::Copy)]
    pub mode: CliStrategy,

    /// Compression format (required only for archive mode)
    #[arg(
        short,
        long,
        value_enum,
        requires_if("mode", "archive"),
        default_value_t = ArchiveFormat::default()
    )]
    pub format: ArchiveFormat,

    /// Compression level (required only for archive mode)
    #[arg(
        short,
        long,
        value_enum,
        requires_if("mode", "archive"),
        default_value_t = Level::default()
    )]
    pub level: Level,

    /// Ignore a specific list of files or directories
    #[arg(short = 'g', long, value_delimiter = ',')]
    ignore: Option<Vec<String>>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CliStrategy {
    Mirror,
    Copy,
    Archive,
}

impl ProcessCommand for AddArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let mut config = manager.load()?;

        let strategy = match self.mode {
            CliStrategy::Mirror => Strategy::Mirror,
            CliStrategy::Copy => Strategy::Copy,
            CliStrategy::Archive => Strategy::Archive {
                format: self.format,
                level: self.level,
            },
        };
        let new_job = Job {
            id: 0,
            source: self.source,
            target: self.target,
            strategy,
            ignore: self.ignore.unwrap_or_default(),
        };
        new_job.validate()?;

        let saved_job = config.add_job(new_job)?;
        manager.save(&config)?;

        println!("✅ New backup job created successfully!");
        println!("  ID       : {}", saved_job.id);
        println!("  Source   : {}", saved_job.source.display());
        println!("  Target   : {}", saved_job.target.display());

        match &saved_job.strategy {
            Strategy::Archive { format, level } => {
                println!(
                    "  Strategy : Archive (Format: {:?}, Level: {:?})",
                    format, level
                );
            }
            Strategy::Mirror => println!("  Strategy : Mirror"),
            Strategy::Copy => println!("  Strategy : Copy"),
        }

        if !saved_job.ignore.is_empty() {
            println!("  Ignore   : {:?}", saved_job.ignore);
        }

        Ok(())
    }
}
