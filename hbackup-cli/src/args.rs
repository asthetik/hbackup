use crate::commands::{ProcessCommand, add::AddArgs, config, delete::DeleteArgs, list::ListArgs};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Supported hbackup commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new backup job to the configuration.
    Add(AddArgs),
    /// Delete backup jobs by id or delete all jobs.
    Delete(DeleteArgs),
    /// List all backup jobs.
    List(ListArgs),
    /// Display the current configuration Path.
    Config,
}

impl Commands {
    pub async fn execute(self) -> anyhow::Result<()> {
        match self {
            Commands::Add(args) => args.run().await,
            Commands::Delete(args) => args.run().await,
            Commands::List(args) => args.run().await,
            Commands::Config => {
                config::run()?;
                Ok(())
            }
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CliStrategy {
    Mirror,
    SimpleCopy,
    Archive,
}
