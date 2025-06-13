use std::error::Error;

use clap::Parser;
use hbackup::commands::{self, Cli, Commands};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let commands = cli
        .commands
        .expect("No command provided. Use --help for more information.");

    match commands {
        Commands::Add { source, target } => {
            commands::add(source, target)?;
        }
        Commands::Run { source, target, id } => {
            if let Some(id) = id {
                commands::run_by_id(id)?;
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
        Commands::Config => {
            commands::config();
        }
    }
    Ok(())
}
