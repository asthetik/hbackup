use std::error::Error;

use clap::Parser;
use hbackup::commands::{self, Cli, Commands};

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(ref commands) = cli.commands {
        println!("{commands:#?}");
    }

    let commands = cli.commands.unwrap();

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
    }
    Ok(())
}
