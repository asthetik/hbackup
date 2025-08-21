mod application;
mod commands;
mod common;
mod file_util;
mod path_util;
mod sysexits;

use crate::commands::{Cli, Commands, EditParams, canonicalize};
use anyhow::Result;
use application::{Job, init_config};
use clap::Parser;
use std::process;

/// Entry point for the hbackup CLI application.
/// Parses command-line arguments and dispatches to the appropriate command handler.
fn main() -> Result<()> {
    let commands = match Cli::parse().commands {
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
            ignore,
        } => {
            commands::add(source, target, compression, level, ignore)?;
        }
        Commands::Run {
            source,
            target,
            compression,
            id,
            level,
            ignore,
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
                    ignore,
                };
                commands::run_job(&job)?;
            } else {
                commands::run()?;
            }
        }
        Commands::List { id, gte, lte } => {
            if let Some(ids) = id {
                commands::list_by_ids(ids);
            } else if let Some(gte) = gte {
                commands::list_by_gte(gte);
            } else if let Some(lte) = lte {
                commands::list_by_lte(lte);
            } else {
                commands::list();
            }
        }
        Commands::Delete { id, all } => {
            commands::delete(id, all)?;
        }
        Commands::Edit {
            id,
            source,
            target,
            compression,
            level,
            ignore,
            clear,
        } => {
            let edit_params = EditParams {
                id,
                source,
                target,
                compression,
                level,
                ignore,
                clear,
            };
            commands::edit(edit_params)?;
        }
        Commands::Config {
            copy,
            reset,
            rollback,
        } => {
            if copy {
                commands::backup_config_file();
            } else if reset {
                commands::reset_config_file();
            } else if rollback {
                commands::rollback_config_file();
            } else {
                commands::config();
            }
        }
    }
    Ok(())
}
