use crate::Result;
use anyhow::bail;
use clap::Args;
use std::io::{self, Write};

use crate::commands::{ProcessCommand, load_config_manager};

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Delete multiple jobs by ids. Cannot be used with --all.
    #[arg(value_delimiter = ',', conflicts_with = "all")]
    pub id: Option<Vec<u32>>,
    /// Delete all jobs. Cannot be used with positional [ID]...
    #[arg(short, long, conflicts_with = "id")]
    pub all: bool,
    /// Skip interactive confirmation when deleting all jobs
    #[arg(short = 'y')]
    pub yes: bool,
}

impl ProcessCommand for DeleteArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let mut config = manager.load()?;

        if config.jobs().is_empty() {
            println!("No jobs to delete");
            return Ok(());
        }

        if self.all {
            if !self.yes {
                confirm_delete_all()?;
            }
            config.reset_jobs();
            manager.save(&config)?;
            println!("All jobs deleted successfully.");
            return Ok(());
        }

        if let Some(ids) = self.id {
            config.delete(ids)?;
            manager.save(&config)?;
            return Ok(());
        }

        bail!("Either --all or --id must be specified.");
    }
}

fn confirm_delete_all() -> Result<()> {
    loop {
        print!("Are you sure you want to delete all jobs? (y/n): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim().to_lowercase().as_str() {
            "y" => return Ok(()),
            "n" => return Ok(()),
            _ => println!("Invalid input. Please enter 'y' or 'n'."),
        }
    }
}
