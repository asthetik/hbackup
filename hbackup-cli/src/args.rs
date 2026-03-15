use crate::commands::{ProcessCommand, add::AddArgs};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Add(AddArgs),
}

impl Commands {
    pub async fn execute(self) -> anyhow::Result<()> {
        match self {
            Commands::Add(args) => args.run().await,
        }
    }
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CliStrategy {
    Mirror,
    SimpleCopy,
    Archive,
}
