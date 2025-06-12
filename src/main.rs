use std::error::Error;

use clap::Parser;
use hbackup::commands::{self, Cli, Commands};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let commands = cli
        .commands
        .expect("No command provided. Use --help for more information.");

    match commands {
        Commands::Create { source, target, id } => {
            commands::create(source, target, id)?;
        }
        Commands::Run => {
            commands::run()?;
        }
        Commands::List => {
            commands::list()?;
        }
        Commands::Delete { id, all } => {
            commands::delete(id, all)?;
        }
    }
    Ok(())
}
