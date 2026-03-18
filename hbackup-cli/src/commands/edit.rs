use crate::Result;
use anyhow::{anyhow, bail};
use clap::Args;
use hbackup_core::model::job::{ArchiveFormat, Level, Strategy};
use std::{fs, path::PathBuf};

use crate::commands::{ProcessCommand, add::CliStrategy, load_config_manager};

#[derive(Args, Debug)]
pub struct EditArgs {
    /// Job ID to edit
    pub id: u32,

    /// New source path (optional)
    #[arg(short, long)]
    pub source: Option<PathBuf>,

    /// New target path (optional)
    #[arg(short, long)]
    pub target: Option<PathBuf>,

    /// New backup mode (optional)
    #[arg(short, long, value_enum)]
    pub mode: Option<CliStrategy>,

    /// New compression format (optional, required only if mode is archive)
    #[arg(short, long, value_enum, requires_if("mode", "archive"))]
    pub format: Option<ArchiveFormat>,

    /// New compression level (optional, required only if mode is archive)
    #[arg(short, long, value_enum, requires_if("mode", "archive"))]
    pub level: Option<Level>,

    /// Ignore a specific list of files or directories
    #[arg(short = 'g', long, value_delimiter = ',')]
    ignore: Option<Vec<String>>,
}

impl ProcessCommand for EditArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let mut config = manager.load()?;

        let job = config
            .get_job_mut(self.id)
            .ok_or_else(|| anyhow!("Job with ID {} not found", self.id))?;

        let mode_opt = self.mode;
        let format_opt = self.format;
        let level_opt = self.level;
        let source_opt = self.source;
        let target_opt = self.target;

        if let Some(source) = source_opt {
            job.source = fs::canonicalize(source)?;
        }
        if let Some(target) = target_opt {
            job.target = fs::canonicalize(target)?;
        }
        if let Some(ignore) = self.ignore {
            job.ignore = ignore;
        }

        let update_existing_archive =
            |format_opt: Option<ArchiveFormat>,
             level_opt: Option<Level>,
             archive: &mut Strategy| {
                if let Strategy::Archive { format, level } = archive {
                    if let Some(new_format) = format_opt {
                        *format = new_format;
                    }
                    if let Some(new_level) = level_opt {
                        *level = new_level;
                    }
                    true
                } else {
                    false
                }
            };

        match mode_opt {
            Some(CliStrategy::Copy) => {
                job.strategy = Strategy::Copy;
            }
            Some(CliStrategy::Mirror) => {
                job.strategy = Strategy::Mirror;
            }
            Some(CliStrategy::Archive) => {
                if !update_existing_archive(format_opt, level_opt, &mut job.strategy)
                    && (format_opt.is_none() && level_opt.is_none())
                {
                    bail!("Both format and level must be provided when changing to archive mode");
                }
                job.strategy = Strategy::Archive {
                    format: format_opt.unwrap_or_default(),
                    level: level_opt.unwrap_or_default(),
                };
            }
            None => {
                if !update_existing_archive(format_opt, level_opt, &mut job.strategy)
                    && (format_opt.is_some() || level_opt.is_some())
                {
                    job.strategy = Strategy::Archive {
                        format: format_opt.unwrap_or_default(),
                        level: level_opt.unwrap_or_default(),
                    };
                }
            }
        }
        manager.save(&config)?;
        println!("✅ Job with ID {} updated successfully!", self.id);

        Ok(())
    }
}
