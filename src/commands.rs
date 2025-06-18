//! Command-line interface definition for hbackup.
use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process;
use std::{error::Error, fs};

use crate::application::{Application, Job, JobList};
use crate::{application, path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Command-line interface definition for hbackup.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub commands: Option<Commands>,
}

/// Supported hbackup commands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new backup job to the configuration.
    Add {
        /// Source file path.
        #[arg(short, long)]
        source: String,
        /// Target file or directory path.
        #[arg(short, long)]
        target: String,
    },
    /// Run backup jobs.
    ///
    /// Usage examples:
    ///   bk run                # Run all jobs
    ///   bk run --id `<id>`      # Run a specific job by id
    ///   bk run `<source>` `<target>`  # Run a one-time backup with given source and target
    Run {
        /// Source file (positional, optional). Must be used with target.
        #[arg(required = false, requires = "target")]
        source: Option<String>,
        /// Target file or directory (positional, optional). Must be used with source.
        #[arg(required = false, requires = "source")]
        target: Option<String>,
        /// Run a specific job by id. Cannot be used with source/target.
        #[arg(long, required = false, conflicts_with_all = ["source", "target"])]
        id: Option<u32>,
    },
    /// List all backup jobs.
    List,
    /// Delete backup jobs by id or delete all jobs.
    Delete {
        /// Delete job by id. Cannot be used with --all.
        #[arg(long, required = false, conflicts_with = "all")]
        id: Option<u32>,
        /// Delete all jobs. Cannot be used with --id.
        #[arg(long, required = false, conflicts_with = "id")]
        all: bool,
    },
    /// Edit a backup job by id. At least one of source/target must be provided.
    Edit {
        /// Edit job by id.
        #[arg(long)]
        id: u32,
        /// New source file or directory path (optional, at least one of source/target required)
        #[arg(short, long, required = false, required_unless_present = "target")]
        source: Option<String>,
        /// New target file or directory path (optional, at least one of source/target required)
        #[arg(short, long, required = false, required_unless_present = "source")]
        target: Option<String>,
    },
    /// Display the absolute path of the configuration file
    Config {
        /// backup the configuration file
        #[arg(long, required = false)]
        copy: bool,
        /// Reset the configuration file and back up the file before resetting
        #[arg(long, required = false)]
        reset: bool,
        /// Rollback the last backed up configuration file
        #[arg(long, required = false)]
        rollback: bool,
    },
}

/// Adds a new backup job to the configuration file.
pub fn add(source: String, target: String) -> Result<()> {
    let source = path::expand_path(&source);
    let target = path::expand_path(&target);
    path::check_path(&source)?;

    let mut app = Application::load_config();
    app.add_job(source, target);
    app.write()?;

    Ok(())
}

/// Runs all backup jobs.
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

/// Runs a backup job by its id.
pub fn run_by_id(id: u32) {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        eprintln!("No jobs are backed up!");
    }
    match jobs.iter().find(|j| j.id == id) {
        Some(job) => match run_job(job) {
            Ok(_) => println!("backed up successfully!"),
            Err(e) => eprintln!(
                "Error: Failed to backup job id: {} from {} to {}\n{}",
                job.id,
                job.source.display(),
                job.target.display(),
                e
            ),
        },
        None => eprintln!("Job with id {id} not found."),
    }
}

/// Runs a one-time backup job with the given source and target.
pub fn run_one_time(source: String, target: String) -> Result<()> {
    let source = path::expand_path(&source);
    let target = path::expand_path(&target);
    path::check_path(&source)?;

    if source.is_dir() {
        if target.exists() && target.is_file() {
            eprintln!("File exists");
            process::exit(1);
        }
        let jobs = get_all_jobs(&source, &target)?;
        for (source, target) in jobs {
            copy_file(&source, &target)?;
        }
    } else {
        copy_file(&source, &target)?;
    }

    Ok(())
}

fn run_job(job: &Job) -> Result<()> {
    path::check_path(&job.source)?;
    if job.source.is_dir() {
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(1);
        }
        let jobs = get_all_jobs(&job.source, &job.target)?;
        for (source, target) in jobs {
            copy_file(&source, &target)?;
        }
    } else {
        copy_file(&job.source, &job.target)?;
    }

    Ok(())
}

/// Lists all backup jobs.
pub fn list() {
    let jobs = Application::get_jobs();
    println!("{}", JobList(jobs));
}

/// Deletes a job by id or deletes all jobs.
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

/// Edits a job by id, updating its source and/or target.
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

/// Prints the absolute path to the configuration file.
pub fn config() {
    println!("config file: {}", application::config_file().display());
}

pub fn backup_config_file() -> Result<()> {
    let config_file = application::config_file();
    let backed_config_file = application::backed_config_file();
    // If the configuration file does not exist, initialize it
    if !config_file.exists() {
        let app = Application::new();
        app.write()?;
    }
    fs::copy(config_file, backed_config_file)
        .with_context(|| "Configuration file backup failed!")?;
    println!("Backup successfully!");
    Ok(())
}

/// Reset the configuration file and back up the file before resetting
pub fn reset_config_file() -> Result<()> {
    let config_file = application::config_file();
    let backed_config_file = application::backed_config_file();
    // Backup the config file if it exists
    if config_file.exists() {
        fs::copy(config_file, backed_config_file)
            .with_context(|| "Configuration file backup failed!")?;
    }
    // Initialize or reset the config file
    let app = Application::new();
    app.write()?;
    Ok(())
}

/// Rollback the last backed up configuration file
pub fn rollback_config_file() -> Result<()> {
    let backed_config_file = application::backed_config_file();
    if !backed_config_file.exists() {
        eprintln!("The backup configuration file does not exist.");
        return Ok(());
    }
    let app = match application::read_backed_config_file() {
        Ok(app) => app,
        Err(e) => {
            eprintln!(
                "Data format conversion error, unable to roll back configuration file.\n{}",
                e
            );
            process::exit(1);
        }
    };
    app.write()?;

    Ok(())
}

fn get_all_jobs(source: &Path, target: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let files = path::get_all_files(source)?;
    let file_name = source.file_name().with_context(|| "Invalid file name")?;
    let mut vec = Vec::new();
    for file in files {
        let sub_path = file.strip_prefix(source)?;
        let target = target.join(file_name).join(sub_path);

        vec.push((file, target));
    }
    Ok(vec)
}

/// copy file from source to target
fn copy_file(source: &Path, target: &Path) -> Result<()> {
    assert!(source.is_file());

    let target_file = if (target.exists() && target.is_dir())
        || (!target.exists() && target.extension().is_none())
    {
        let file_name = source.file_name().with_context(|| "Invalid file name")?;
        target.join(file_name)
    } else {
        target.into()
    };

    if let Some(parent) = target_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::copy(source, &target_file)?;

    Ok(())
}
