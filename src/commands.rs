use std::{error::Error, fs};

use clap::{Parser, Subcommand};

use crate::application::{Application, Job, JobList};
use crate::{application, path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Option<Commands>,
}

/// hbackup commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// add a backup job
    Add {
        /// source file
        #[arg(short, long)]
        source: String,
        /// target file or directory
        #[arg(short, long)]
        target: String,
    },
    /// Run backup jobs.
    ///
    /// Usage:
    ///   bk run                # Run all jobs
    ///   bk run --id <id>      # Run a specific job by id
    ///   bk run <source> <target>  # Run a one-time backup with given source and target
    Run {
        /// source file (positional, optional)
        #[arg(required = false, requires = "target")]
        source: Option<String>,
        /// target file (positional, optional)
        #[arg(required = false, requires = "source")]
        target: Option<String>,
        #[arg(long, required = false, conflicts_with_all = ["source", "target"])]
        id: Option<u32>,
    },
    /// list all jobs
    List,
    /// delete jobs
    Delete {
        /// delete job by id
        #[arg(long, required = false, conflicts_with = "all")]
        id: Option<u32>,
        /// delete all jobs
        #[arg(long, required = false, conflicts_with = "id")]
        all: bool,
    },
    /// edit a job
    Edit {
        /// edit job by id
        #[arg(long)]
        id: u32,
        /// source file (optional, at least one of source/target required)
        #[arg(short, long, required = false, required_unless_present = "target")]
        source: Option<String>,
        /// target file (optional, at least one of source/target required)
        #[arg(short, long, required = false, required_unless_present = "source")]
        target: Option<String>,
    },
    /// Display the configuration file path
    Config,
}

pub fn add(source: String, target: String) -> Result<()> {
    let source = path::expand_path(&source);
    let target = path::expand_path(&target);
    path::check_path(&source)?;

    let mut app = Application::load_config();
    app.add_job(source, target);
    app.write()?;

    Ok(())
}

pub fn run() -> Result<()> {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        println!("No jobs are backed up!");
        return Ok(());
    }
    for job in jobs {
        if let Err(e) = run_job(&job) {
            eprintln!("Failed to run job id {}: {}", job.id, e);
        }
    }
    Ok(())
}

pub fn run_by_id(id: u32) -> Result<()> {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        println!("No jobs are backed up!");
        return Ok(());
    }
    match jobs.iter().find(|j| j.id == id) {
        Some(job) => run_job(job)?,
        None => println!("Job with id {id} not found."),
    }
    Ok(())
}

pub fn run_one_time(source: String, target: String) -> Result<()> {
    let source = path::expand_path(&source);
    let mut target = path::expand_path(&target);
    path::check_path(&source)?;

    let target_file = if (target.exists() && target.is_dir())
        || (!target.exists() && target.extension().is_none())
    {
        let file_name = source.file_name().ok_or("invalid file name")?;
        target.push(file_name);
        target
    } else {
        target
    };

    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::copy(&source, &target_file) {
        Ok(_) => println!(
            "Job from {} to {} backed up successfully.",
            source.display(),
            target_file.display()
        ),
        Err(e) => eprintln!(
            "Failed to backup job from {} to {}: {}",
            source.display(),
            target_file.display(),
            e
        ),
    }
    Ok(())
}

fn run_job(job: &Job) -> Result<()> {
    let target_file = if (job.target.exists() && job.target.is_dir())
        || (!job.target.exists() && job.target.extension().is_none())
    {
        let file_name = job.source.file_name().ok_or("invalid file name")?;
        let mut target = job.target.clone();
        target.push(file_name);
        target
    } else {
        job.target.clone()
    };

    if let Some(parent) = target_file.parent() {
        fs::create_dir_all(parent)?;
    }

    match fs::copy(&job.source, &target_file) {
        Ok(_) => println!(
            "Task id: {} from {} to {} backed up successfully.",
            job.id,
            job.source.display(),
            target_file.display()
        ),
        Err(e) => eprintln!(
            "Failed to backup job id: {} from {} to {}: {}",
            job.id,
            job.source.display(),
            target_file.display(),
            e
        ),
    }
    Ok(())
}

pub fn list() {
    let jobs = Application::get_jobs();
    println!("{}", JobList(jobs));
}

pub fn delete(id: Option<u32>, all: bool) -> Result<()> {
    if all {
        let mut app = Application::load_config();
        app.reset_jobs();
        app.write()?;
        println!("All jobs deleted successfully.");
    } else if let Some(id) = id {
        let mut app = Application::load_config();
        match app.remove_job(id) {
            Some(_) => {
                app.write()?;
                println!("Job with id {id} deleted successfully.");
            }
            None => println!("Job deletion failed. Job with id {id} cannot be found."),
        }
    } else {
        return Err("Either --all or --id must be specified.".into());
    }
    Ok(())
}

pub fn edit(id: u32, source: Option<String>, target: Option<String>) -> Result<()> {
    let source = source.map(|path| path::expand_path(&path));
    if let Some(ref file_path) = source {
        path::check_path(file_path)?;
    }
    let target = target.map(|path| path::expand_path(&path));

    let mut app = Application::load_config();
    if app.jobs.is_empty() {
        println!("Job with id {id} not found.");
        return Ok(());
    }
    if let Some(job) = app.jobs.iter_mut().find(|j| j.id == id) {
        if let Some(path) = source {
            job.source = path;
        }
        if let Some(path) = target {
            job.target = path;
        }
        app.write()?;
        println!("Job with id {id} edited successfully.");
    } else {
        println!("Job with id {id} not found.");
    }
    Ok(())
}

pub fn config() {
    println!("config file: {}", application::config_file().display());
}
