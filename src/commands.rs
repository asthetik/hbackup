//! Command-line interface definition for hbackup.
//!
//! This module defines all CLI commands, their arguments, and the logic for handling
//! backup jobs, including add, run, list, delete, edit, and configuration management.

use crate::application::{self, Application, CompressFormat, Job, JobList, Level};
use crate::file_util;
use crate::path_util;
use crate::{Result, sysexits};
use anyhow::Context;
use anyhow::anyhow;
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use walkdir::WalkDir;

/// Command-line interface definition for hbackup.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub(crate) commands: Option<Commands>,
}

/// Supported hbackup commands.
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Add a new backup job to the configuration.
    Add {
        /// Source file path.
        #[arg(short, long)]
        source: PathBuf,
        /// Target file or directory path.
        #[arg(short, long)]
        target: PathBuf,
        /// Compression format.
        #[arg(short, long)]
        compression: Option<CompressFormat>,
        #[arg(short, long, requires = "compression")]
        level: Option<Level>,
    },
    /// Run backup jobs.
    Run {
        /// Source file (positional, optional). Must be used with target.
        #[arg(required = false, requires = "target")]
        source: Option<PathBuf>,
        /// Target file or directory (positional, optional). Must be used with source.
        #[arg(required = false, requires = "source")]
        target: Option<PathBuf>,
        /// Compression format.
        #[arg(required = false)]
        compression: Option<CompressFormat>,
        /// Compression level
        #[arg(required = false, requires = "compression")]
        level: Option<Level>,
        /// Job id(s) to run.
        #[arg(long, required = false, value_delimiter = ',', conflicts_with_all = ["source", "target", "compression"])]
        id: Option<Vec<u32>>,
    },
    /// List all backup jobs.
    List,
    /// Delete backup jobs by id or delete all jobs.
    Delete {
        /// Delete multiple job by id. Cannot be used with --all.
        #[arg(long, required = false, value_delimiter = ',', conflicts_with = "all")]
        id: Option<Vec<u32>>,
        /// Delete all jobs. Cannot be used with --id.
        #[arg(long, required = false, conflicts_with = "id")]
        all: bool,
    },
    /// Edit a backup job by id. At least one of source/target must be provided.
    Edit {
        /// Edit job by id.
        #[arg(long)]
        id: u32,
        /// New source file or directory path
        #[arg(short, long, required = false, required_unless_present_any = ["target", "compression", "no_compression", "level", "no_level"])]
        source: Option<PathBuf>,
        /// New target file or directory path
        #[arg(short, long, required = false, required_unless_present_any = ["source", "compression", "no_compression", "level", "no_level"])]
        target: Option<PathBuf>,
        /// Compression format.
        #[arg(short, long, required = false, required_unless_present_any = ["source", "target", "no_compression", "level", "no_level"], conflicts_with_all = ["no_compression"])]
        compression: Option<CompressFormat>,
        /// Clear compression format
        #[arg(short = 'C', long, required = false, required_unless_present_any = ["source", "target", "compression", "level", "no_level"], conflicts_with_all = ["compression"])]
        no_compression: bool,
        /// Compression level
        #[arg(short, long, required = false, required_unless_present_any = ["source", "target", "compression", "no_compression", "no_level"], conflicts_with_all = ["no_level"] )]
        level: Option<Level>,
        /// Clear compression level
        #[arg(short = 'L', long, required = false, required_unless_present_any = ["source", "target", "compression", "no_compression", "level"], conflicts_with_all = ["level"] )]
        no_level: bool,
    },
    /// Display the absolute path of the configuration file and manage config backup/reset/rollback.
    Config {
        /// Backup the configuration file.
        #[arg(long, required = false)]
        copy: bool,
        /// Reset the configuration file and back up the file before resetting.
        #[arg(long, required = false)]
        reset: bool,
        /// Rollback the last backed up configuration file.
        #[arg(long, required = false)]
        rollback: bool,
    },
}

/// Adds a new backup job to the configuration file.
///
/// # Arguments
/// * `source` - The source file or directory path.
/// * `target` - The target file or directory path.
/// * `comp` - Optional compression format.
///
/// # Errors
/// Returns an error if the source path is invalid or the job cannot be saved.
pub(crate) fn add(
    source: PathBuf,
    target: PathBuf,
    comp: Option<CompressFormat>,
    level: Option<Level>,
) -> Result<()> {
    let source = canonicalize(source);
    let target = canonicalize(target);
    path_util::check_path(&source)?;

    let mut app = Application::load_config();
    app.add_job(source, target, comp, level);
    app.write()?;

    Ok(())
}

/// Runs all backup jobs defined in the configuration.
///
/// # Errors
/// Returns an error if any job fails to run.
pub(crate) fn run() -> Result<()> {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        println!("No jobs are backed up!");
    } else if jobs.len() == 1 {
        run_job(&jobs[0])?;
    } else {
        run_jobs(jobs)?;
    }
    Ok(())
}

/// Runs multiple backup jobs concurrently.
pub(crate) fn run_jobs(jobs: Vec<Job>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let mut set = tokio::task::JoinSet::new();
        for job in jobs {
            set.spawn(async move {
                if let Err(e) = run_job_async(&job).await {
                    eprintln!("Failed to run job with id {}: {}\n", job.id, e);
                }
            });
        }
        while let Some(res) = set.join_next().await {
            if let Err(e) = res {
                eprintln!("Failed to run job: {e}\n");
            }
        }
    });

    Ok(())
}

/// Runs a backup job by its id.
///
/// # Arguments
/// * `id` - The job id to run.
///
/// Exits the process with an error code if the job is not found or fails.
pub(crate) fn run_by_id(ids: Vec<u32>) {
    let jobs = Application::get_jobs();
    if jobs.is_empty() {
        eprintln!("No jobs are backed up!");
        process::exit(sysexits::EX_DATAERR);
    }
    for id in ids {
        match jobs.iter().find(|j| j.id == id) {
            Some(job) => {
                if let Err(e) = run_job(job) {
                    eprintln!("Failed to run job with id {}: {}\n", job.id, e);
                    process::exit(sysexits::EX_IOERR);
                }
            }
            None => {
                eprintln!("Job with id {id} not found.");
                process::exit(sysexits::EX_DATAERR);
            }
        }
    }
}

/// Runs a backup job (single file or directory copy, with optional compression).
///
/// # Arguments
/// * `job` - The job to execute.
///
/// # Errors
/// Returns an error if the copy or compression fails.
pub(crate) fn run_job(job: &Job) -> Result<()> {
    if let Some(ref format) = job.compression {
        let level = job.level.as_ref().unwrap_or(&Level::Default);
        file_util::compression(&job.source, &job.target, format, level)?;
    } else if job.source.is_dir() {
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(sysexits::EX_CANTCREAT);
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

/// Runs a backup job (single file or directory copy, with optional compression).
///
/// # Arguments
/// * `job` - The job to execute.
///
/// # Errors
/// Returns an error if the copy or compression fails.
pub(crate) async fn run_job_async(job: &Job) -> Result<()> {
    if let Some(ref format) = job.compression {
        let level = job.level.as_ref().unwrap_or(&Level::Default);
        let src = job.source.clone();
        let tgt = job.target.clone();
        let fmt = format.clone();
        let lvl = level.clone();
        tokio::task::spawn_blocking(move || file_util::compression(&src, &tgt, &fmt, &lvl))
            .await??;
    } else if job.source.is_dir() {
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(sysexits::EX_CANTCREAT);
        }
        let jobs = get_all_jobs(&job.source, &job.target)?;
        let mut tasks = FuturesUnordered::new();
        for (source, target) in jobs {
            tasks.push(tokio::fs::copy(source, target));
        }
        while let Some(res) = tasks.next().await {
            res?;
        }
    } else {
        tokio::fs::copy(&job.source, &job.target).await?;
    }
    Ok(())
}

/// Lists all backup jobs in the configuration.
pub(crate) fn list() {
    let jobs = Application::get_jobs();
    println!("{}", JobList(jobs));
}

/// Deletes a job by id or deletes all jobs.
///
/// # Arguments
/// * `id` - Optional job id to delete.
/// * `all` - If true, delete all jobs.
///
/// # Errors
/// Returns an error if neither `id` nor `all` is specified, or if deletion fails.
pub(crate) fn delete(id: Option<Vec<u32>>, all: bool) -> Result<()> {
    if all {
        let mut app = Application::load_config();
        app.reset_jobs();
        app.write()?;
        println!("All jobs deleted successfully.");
    } else if let Some(ids) = id {
        let mut app = Application::load_config();
        for id in ids {
            match app.remove_job(id) {
                Some(_) => {
                    app.write()?;
                    println!("Job with id {id} deleted successfully.");
                }
                None => println!("Job deletion failed. Job with id {id} cannot be found."),
            }
        }
    } else {
        return Err(anyhow!("Either --all or --id must be specified."));
    }
    Ok(())
}

/// Edits a job by id, updating its source, target, and/or compression settings.
///
/// # Arguments
/// * `id` - The job id to edit.
/// * `source` - Optional new source path. If provided, replaces the job's source.
/// * `target` - Optional new target path. If provided, replaces the job's target.
/// * `compression` - Optional new compression format. If provided and `no_compression` is false, replaces the job's compression.
/// * `no_compression` - If true, clears the job's compression format (takes precedence over `compression`).
///
/// # Errors
/// Returns an error if the job is not found or the new path is invalid.
pub(crate) fn edit(
    id: u32,
    source: Option<PathBuf>,
    target: Option<PathBuf>,
    compression: Option<CompressFormat>,
    no_compression: bool,
    level: Option<Level>,
    no_level: bool,
) -> Result<()> {
    let source = source.map(canonicalize);
    if let Some(ref file_path) = source {
        path_util::check_path(file_path)?;
    }
    let target = target.map(canonicalize);

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
        if no_compression {
            job.compression = None;
            job.level = None;
        } else if compression.is_some() {
            job.compression = compression;
        }
        if no_level {
            job.level = None;
        } else if level.is_some() && job.compression.is_none() {
            eprintln!(
                "The compression format is not set, and the compression level cannot be updated."
            );
            process::exit(1);
        } else if level.is_some() {
            job.level = level;
        }
        app.write()?;
        println!("Job with id {id} edited successfully.");
    } else {
        println!("Job with id {id} not found.");
    }
    Ok(())
}

/// Prints the absolute path to the configuration file.
pub(crate) fn config() {
    println!("config file: {}", application::config_file().display());
}

/// Back up the configuration file to a backup location.
///
/// # Errors
/// Returns an error if the backup fails.
pub(crate) fn backup_config_file() -> Result<()> {
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

/// Reset the configuration file and back up the file before resetting.
///
/// # Errors
/// Returns an error if the reset or backup fails.
pub(crate) fn reset_config_file() -> Result<()> {
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

/// Rollback the last backed up configuration file.
///
/// # Errors
/// Returns an error if the backup does not exist or rollback fails.
pub(crate) fn rollback_config_file() -> Result<()> {
    let backed_config_file = application::backed_config_file();
    if !backed_config_file.exists() {
        eprintln!("The backup configuration file does not exist.");
        return Ok(());
    }
    let app = match application::read_backed_config_file() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Data format conversion error, unable to roll back configuration file\n{e}");
            process::exit(sysexits::EX_IOERR);
        }
    };
    app.write()?;

    Ok(())
}

/// Recursively collects all files in a directory for backup, mapping source to target paths.
///
/// # Arguments
/// * `source` - The source directory.
/// * `target` - The target directory.
///
/// # Returns
/// A vector of (source_file, target_file) pairs.
///
/// # Errors
/// Returns an error if directory traversal fails.
fn get_all_jobs(source: &Path, target: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let prefix = source.parent().unwrap_or(Path::new(""));
    let mut vec = vec![];
    for entry in WalkDir::new(source) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let rel: PathBuf = path
                .strip_prefix(prefix)
                .expect("strip_prefix failed")
                .into();
            let target_path = target.join(rel);
            vec.push((path.to_path_buf(), target_path));
        }
    }
    Ok(vec)
}

/// Copy file from source to target, creating parent directories if needed.
///
/// # Arguments
/// * `source` - Path to the source file.
/// * `target` - Path to the target file or directory.
///
/// # Errors
/// Returns an error if the copy fails.
fn copy_file(source: &Path, target: &Path) -> Result<()> {
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

/// Returns the canonical, absolute form of the path with all intermediate
/// components normalized and symbolic links resolved.
///
/// # Arguments
/// * `path` - The path to canonicalize.
///
/// # Panics
/// Exits the process if the path is invalid.
pub(crate) fn canonicalize(path: PathBuf) -> PathBuf {
    let source = &path;
    match source.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("The path or file '{source:?}' is invalid\n{e}");
            process::exit(1);
        }
    }
}
