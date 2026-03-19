use crate::Result;
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
            println!("Checked: No backup jobs defined. Nothing to delete.");
            return Ok(());
        }

        if self.all {
            if !self.yes && !confirm_delete_all()? {
                println!("Operation cancelled. No jobs were deleted.");
                return Ok(());
            }

            let count = config.jobs().len();
            config.reset_jobs();
            manager.save(&config)?;
            println!("🗑️  Deleted all backup jobs (Total: {}).", count);
            return Ok(());
        }

        if let Some(ids) = self.id {
            config.delete(ids.clone())?;
            manager.save(&config)?;
            println!("✅ Successfully removed jobs with ID(s): {:?}", ids);
            return Ok(());
        }

        Ok(())
    }
}

fn confirm_delete_all() -> Result<bool> {
    loop {
        print!("⚠️  Are you sure you want to delete ALL backup jobs? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice.is_empty() || choice == "n" {
            return Ok(false);
        }
        if choice == "y" {
            return Ok(true);
        }
        println!("Invalid input. Please enter 'y' for yes or 'n' for no.");
    }
}
