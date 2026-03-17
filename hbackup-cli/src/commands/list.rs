use anyhow::Result;
use clap::Args;
use hbackup_core::model::job::Job;

use crate::commands::{ProcessCommand, load_config_manager};

#[derive(Args, Debug)]
pub struct ListArgs {
    /// List jobs by ids.
    #[arg(short, long, required = false, value_delimiter = ',', conflicts_with_all = ["gte", "lte"])]
    id: Option<Vec<u32>>,
    /// List jobs by id greater than or equal to.
    #[arg(short = 'g', long, required = false, conflicts_with_all = ["id", "lte"])]
    gte: Option<u32>,
    /// List jobs by id less than or equal to.
    #[arg(short = 'l', long, required = false, conflicts_with_all = ["id", "gte"])]
    lte: Option<u32>,
}

impl ProcessCommand for ListArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let config = manager.load()?;
        let jobs = if let Some(ids) = self.id {
            config.list_by_ids(&ids)
        } else if let Some(id) = self.gte {
            config.list_by_gte(id)
        } else if let Some(id) = self.lte {
            config.list_by_lte(id)
        } else {
            config.jobs().iter().collect()
        };

        if !jobs.is_empty() {
            let display = display_jobs(jobs);
            println!("{display}");
        }

        Ok(())
    }
}

fn display_jobs(jobs: Vec<&Job>) -> String {
    if jobs.is_empty() {
        return String::new();
    }
    let mut s = String::from('[');
    for job in jobs {
        s.push_str(&format!(
            "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\",\n    strategy: {:?}",
            job.id,
            job.source.display(),
            job.target.display(),
            job.strategy
        ));
        s.push_str("\n},");
    }
    s.pop();
    s.push(']');
    s
}
