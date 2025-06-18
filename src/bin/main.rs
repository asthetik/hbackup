use clap::Parser;
use hbackup::commands::{self, Cli, Commands};
use std::error::Error;
use std::process;
use hbackup::path;

/// Entry point for the hbackup CLI application.
/// Parses command-line arguments and dispatches to the appropriate command handler.
fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let commands = match cli.commands {
        Some(commands) => commands,
        None => {
            eprintln!("bk requires at least one command to execute. See 'bk --help' for usage.");
            process::exit(1);
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
                let source = path::expand_path(&source);
                let target = path::expand_path(&target);
                path::check_path(&source)?;
                commands::run_job(&source, &target)?;
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
