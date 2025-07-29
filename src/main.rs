mod application;
mod commands;
mod common;
mod file_util;
mod path_util;
mod sysexits;

use anyhow::Result;
use application::{Job, init_config};
use clap::Parser;
use commands::{Cli, Commands, canonicalize};
use std::process;

/// Entry point for the hbackup CLI application.
/// Parses command-line arguments and dispatches to the appropriate command handler.
fn main() -> Result<()> {
    let cli = Cli::parse();

    let commands = match cli.commands {
        Some(commands) => commands,
        None => {
            eprintln!("bk requires at least one command to execute. See 'bk --help' for usage.");
            process::exit(sysexits::EX_KEYWORD);
        }
    };

    init_config();

    match commands {
        Commands::Add {
            source,
            target,
            compression,
            level,
        } => {
            commands::add(source, target, compression, level)?;
        }
        Commands::Run {
            source,
            target,
            compression,
            id,
            level,
        } => {
            if let Some(ids) = id {
                commands::run_by_id(ids);
            } else if let (Some(source), Some(target)) = (source, target) {
                let source = canonicalize(source);
                let target = canonicalize(target);
                path_util::check_path(&source)?;

                // The temporary job id is set to 0
                let job = Job {
                    id: 0,
                    source,
                    target,
                    compression,
                    level,
                };
                commands::run_job(&job)?;
            } else {
                commands::run()?;
            }
        }
        Commands::List => {
            commands::list();
        }
        Commands::Delete { id, all } => {
            commands::delete(id, all)?;
        }
        Commands::Edit {
            id,
            source,
            target,
            compression,
            no_compression,
            level,
            no_level,
        } => {
            commands::edit(
                id,
                source,
                target,
                compression,
                no_compression,
                level,
                no_level,
            )?;
        }
        Commands::Config {
            copy,
            reset,
            rollback,
        } => {
            if (copy as u32 + reset as u32 + rollback as u32) >= 2 {
                eprintln!(
                    "You cannot specify the --copy, --reset, and --rollback options at the same time. Please choose only one of them."
                );
            } else if copy {
                commands::backup_config_file()?;
            } else if reset {
                commands::reset_config_file()?;
            } else if rollback {
                commands::rollback_config_file()?;
            } else {
                commands::config();
            }
        }
    }
    Ok(())
}
