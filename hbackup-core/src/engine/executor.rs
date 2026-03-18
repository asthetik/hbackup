use crate::{
    error::Result,
    model::job::{Job, Strategy},
    pipeline::mirror::SyncExecutor,
};

pub fn execute_single(job: Job) -> Result<()> {
    match job.strategy {
        Strategy::Copy | Strategy::Mirror => {
            let executor = SyncExecutor::new(job.source, job.target, job.ignore);
            executor.run(job.strategy)?;
        }

        Strategy::Archive { .. } => {
            todo!()
        }
    }

    Ok(())
}

pub fn execute_all(jobs: Vec<Job>) -> Result<()> {
    for job in jobs.into_iter() {
        execute_single(job)?;
    }
    Ok(())
}
