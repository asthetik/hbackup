use std::{error::Error, fs};

use clap::{Parser, Subcommand};

use crate::application::Application;
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
    /// create a backup task
    Create {
        /// source file
        #[arg(short, long)]
        source: String,
        /// target file or directory
        #[arg(short, long)]
        target: String,
    },
    /// run all backup task
    Run,
    /// list all tasks
    List,
    /// delete tasks
    Delete {
        /// delete task by id
        #[arg(short, long, required = false, conflicts_with = "all")]
        id: Option<u32>,
        /// delete all tasks
        #[arg(long, required = false, conflicts_with = "id")]
        all: bool,
    },
    /// Display the configuration file path
    Config,
}

pub fn create(source: String, target: String) -> Result<()> {
    let source = path::expand_path(&source);
    let target = path::expand_path(&target);
    path::check_path(&source)?;

    let mut app = Application::load_config();
    app.add_job(source, target)?;
    app.write()?;

    Ok(())
}

pub fn run() -> Result<()> {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        println!("No tasks are backed up!");
        return Ok(());
    }
    for job in jobs {
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
                "Failed to backup task id: {} from {} to {}: {}",
                job.id,
                job.source.display(),
                target_file.display(),
                e
            ),
        }
    }
    Ok(())
}

pub fn list() {
    let jobs = Application::get_jobs();
    println!("{jobs:#?}");
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

pub fn config() {
    println!("config file: {}", application::config_file().display());
}
