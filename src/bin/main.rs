use clap::Parser;
use hbackup::application::Job;
use hbackup::commands::{self, canonicalize, Cli, Commands};
use hbackup::Result;
use hbackup::{path, sysexits};
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

    match commands {
        Commands::Add { source, target } => {
            commands::add(source, target)?;
        }
        Commands::Run { source, target, id } => {
            if let Some(id) = id {
                commands::run_by_id(id);
            } else if let (Some(source), Some(target)) = (source, target) {
                let source = canonicalize(source);
                let target = canonicalize(target);
                path::check_path(&source)?;
                // The temporary job id is set to 0
                let job = Job {
                    id: 0,
                    source,
                    target,
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
        Commands::Edit { id, source, target } => {
            commands::edit(id, source, target)?;
        }
        Commands::Config {
            copy,
            reset,
            rollback,
        } => {
            if (copy as u32 + reset as u32 + rollback as u32) >= 2 {
                eprintln!("You cannot specify the --copy, --reset, and --rollback options at the same time. Please choose only one of them.");
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
