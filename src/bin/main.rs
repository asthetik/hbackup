use clap::Parser;
use hbackup::commands::{self, Cli, Commands};
use std::error::Error;
use std::process;

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
                commands::run_one_time(source, target)?;
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
        Commands::Config { copy, reset } => {
            if copy && reset {
                return Err("Cannot specify both --copy and --reset at the same time".into());
            } else if copy {
                commands::backup_config_file()?;
            } else if reset {
                commands::reset_config_file()?;
            } else {
                commands::config();
            }
        }
    }
    Ok(())
}
