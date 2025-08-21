//! Command-line interface definition for hbackup.
//!
//! This module defines all CLI commands, their arguments, and the logic for handling
//! backup jobs, including add, run, list, delete, edit, and configuration management.

use crate::application::{self, Application, CompressFormat, Job, Level};
use crate::file_util;
use crate::path_util;
use crate::{Result, sysexits};
use anyhow::Context;
use anyhow::anyhow;
use clap::ValueEnum;
use clap::{Parser, Subcommand};
use futures::stream::{FuturesUnordered, StreamExt};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tokio::runtime::Builder;
use walkdir::WalkDir;

/// Fields that can be cleared in the edit command
#[derive(Debug, Clone, ValueEnum)]
pub(crate) enum ClearField {
    /// Clear compression format
    Compression,
    /// Clear compression level
    Level,
    /// Clear ignore list
    Ignore,
}

/// Command-line interface definition for hbackup.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub commands: Option<Commands>,
}

/// Supported hbackup commands.
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Add a new backup job to the configuration.
    Add {
        /// Source file or directory path.
        source: PathBuf,
        /// Target file or directory path.
        target: PathBuf,
        /// Compression format.
        #[arg(short, long)]
        compression: Option<CompressFormat>,
        #[arg(short, long, requires = "compression")]
        level: Option<Level>,
        /// Ignore a specific list of files or directories
        #[arg(short = 'g', long, value_delimiter = ',')]
        ignore: Option<Vec<String>>,
    },
    /// Run backup jobs.
    Run {
        /// Source file or directory path (positional, optional). Must be used with target.
        #[arg(required = false, requires = "target")]
        source: Option<PathBuf>,
        /// Target file or directory path (positional, optional). Must be used with source.
        #[arg(required = false, requires = "source")]
        target: Option<PathBuf>,
        /// Compression format.
        #[arg(short, long, required = false)]
        compression: Option<CompressFormat>,
        /// Compression level
        #[arg(short, long, required = false, requires = "compression")]
        level: Option<Level>,
        /// Job id(s) to run.
        #[arg(short, long, required = false, value_delimiter = ',', conflicts_with_all = ["source", "target", "compression"])]
        id: Option<Vec<u32>>,
        /// Ignore a specific list of files or directories
        #[arg(short = 'g', long, value_delimiter = ',')]
        ignore: Option<Vec<String>>,
    },
    /// List all backup jobs.
    List {
        /// List jobs by ids.
        #[arg(short, long, required = false, value_delimiter = ',', conflicts_with_all = ["gte", "lte"])]
        id: Option<Vec<u32>>,
        /// List jobs by id greater than or equal to.
        #[arg(short = 'g', long, required = false, conflicts_with_all = ["id", "lte"])]
        gte: Option<u32>,
        /// List jobs by id less than or equal to.
        #[arg(short = 'l', long, required = false, conflicts_with_all = ["id", "gte"])]
        lte: Option<u32>,
    },
    /// Delete backup jobs by id or delete all jobs.
    Delete {
        /// Delete multiple jobs by ids. Cannot be used with --all.
        #[arg(value_delimiter = ',', conflicts_with = "all")]
        id: Option<Vec<u32>>,
        /// Delete all jobs. Cannot be used with --id.
        #[arg(short, long, conflicts_with = "id")]
        all: bool,
    },
    /// Edit a backup job by id. At least one of source/target/compression/level/ignore/clear must be provided.
    Edit {
        /// Edit job by id.
        id: u32,
        /// New source file or directory path
        #[arg(short, long, required_unless_present_any = ["target", "compression", "level", "ignore", "clear"])]
        source: Option<PathBuf>,
        /// New target file or directory path
        #[arg(short, long, required_unless_present_any = ["source", "compression", "level", "ignore", "clear"])]
        target: Option<PathBuf>,
        /// Compression format
        #[arg(short, long, required_unless_present_any = ["source", "target", "level", "ignore", "clear"])]
        compression: Option<CompressFormat>,
        /// Compression level
        #[arg(short, long, required_unless_present_any = ["source", "target", "compression", "ignore", "clear"])]
        level: Option<Level>,
        /// Ignore a specific list of files or directories
        #[arg(short = 'g', long, value_delimiter = ',', required_unless_present_any = ["source", "target", "compression", "level", "clear"])]
        ignore: Option<Vec<String>>,
        /// Clear specified fields (comma-separated: compression,level,ignore)
        #[arg(long, value_delimiter = ',', required_unless_present_any = ["source", "target", "compression", "level", "ignore"])]
        clear: Option<Vec<ClearField>>,
    },
    /// Display the absolute path of the configuration file and manage config backup/reset/rollback.
    Config {
        /// Backup the configuration file.
        #[arg(short = 'c', long, required = false, conflicts_with_all = ["reset", "rollback"])]
        copy: bool,
        /// Reset the configuration file and back up the file before resetting.
        #[arg(short = 'r', long, required = false, conflicts_with_all = ["copy", "rollback"])]
        reset: bool,
        /// Rollback the last backed up configuration file.
        #[arg(short = 'R', long, required = false, conflicts_with_all = ["copy", "reset"])]
        rollback: bool,
    },
}

/// Parameters for editing a backup job
pub(crate) struct EditParams {
    pub id: u32,
    pub source: Option<PathBuf>,
    pub target: Option<PathBuf>,
    pub compression: Option<CompressFormat>,
    pub level: Option<Level>,
    pub ignore: Option<Vec<String>>,
    pub clear: Option<Vec<ClearField>>,
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
    ignore: Option<Vec<String>>,
) -> Result<()> {
    let source = canonicalize(source);
    let target = canonicalize(target);
    path_util::check_path(&source)?;

    let mut app = Application::load_config();
    app.add_job(source, target, comp, level, ignore);
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
    let rt = Builder::new_multi_thread().enable_all().build()?;

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
    let mut vec = vec![];
    for id in ids {
        match jobs.iter().find(|j| j.id == id) {
            Some(job) => {
                vec.push(job.clone());
            }
            None => {
                eprintln!("Job with id {id} not found.");
                process::exit(sysexits::EX_DATAERR);
            }
        }
    }
    assert!(!vec.is_empty(), "No jobs found to run");
    if vec.len() == 1 {
        if let Err(e) = run_job(&vec[0]) {
            eprintln!("Failed to run job with id {}: {e}\n", vec[0].id);
            process::exit(sysexits::EX_IOERR);
        }
    } else if let Err(e) = run_jobs(vec) {
        eprintln!("Failed to run jobs: {e}\n");
        process::exit(sysexits::EX_IOERR);
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
        file_util::compression(&job.source, &job.target, format, level, &job.ignore)?;
    } else if job.source.is_dir() {
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(sysexits::EX_CANTCREAT);
        }
        let jobs = get_jobs(&job.source, &job.target, &job.ignore)?;
        let rt = Builder::new_multi_thread().enable_all().build()?;
        rt.block_on(async {
            let mut tasks = FuturesUnordered::new();
            for (source, target) in jobs {
                tasks.push(copy_file_async(source, target));
            }
            while let Some(res) = tasks.next().await {
                res?;
            }
            Ok::<(), anyhow::Error>(())
        })?;
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
        let ignore = job.ignore.clone();
        tokio::task::spawn_blocking(move || {
            file_util::compression(&src, &tgt, &fmt, &lvl, &ignore)
        })
        .await??;
    } else if job.source.is_dir() {
        if job.target.exists() && job.target.is_file() {
            eprintln!("File exists");
            process::exit(sysexits::EX_CANTCREAT);
        }
        let jobs = get_jobs(&job.source, &job.target, &job.ignore)?;
        let mut tasks = FuturesUnordered::new();
        for (source, target) in jobs {
            tasks.push(copy_file_async(source, target));
        }
        while let Some(res) = tasks.next().await {
            res?;
        }
    } else {
        copy_file_async(job.source.clone(), job.target.clone()).await?;
    }
    Ok(())
}

/// Lists all backup jobs in the configuration.
pub(crate) fn list() {
    let jobs = Application::get_jobs();
    println!("{}", display_jobs(jobs));
}

/// Lists backup jobs by their IDs.
pub(crate) fn list_by_ids(ids: Vec<u32>) {
    let jobs = Application::get_jobs()
        .into_iter()
        .filter(|job| ids.contains(&job.id))
        .collect();
    println!("{}", display_jobs(jobs));
}

/// Lists backup jobs by their IDs.
pub(crate) fn list_by_gte(id: u32) {
    let jobs = Application::get_jobs()
        .into_iter()
        .filter(|job| job.id >= id)
        .collect();
    println!("{}", display_jobs(jobs));
}

/// Lists backup jobs by their IDs.
pub(crate) fn list_by_lte(id: u32) {
    let jobs = Application::get_jobs()
        .into_iter()
        .filter(|job| job.id <= id)
        .collect();
    println!("{}", display_jobs(jobs));
}

fn display_jobs(jobs: Vec<Job>) -> String {
    if jobs.is_empty() {
        return String::new();
    }
    let mut s = String::from('[');
    for job in jobs {
        let comp = match job.compression {
            Some(CompressFormat::Gzip) => "Gzip",
            Some(CompressFormat::Zip) => "Zip",
            Some(CompressFormat::Sevenz) => "Sevenz",
            Some(CompressFormat::Zstd) => "Zstd",
            Some(CompressFormat::Bzip2) => "Bzip2",
            Some(CompressFormat::Xz) => "Xz",
            Some(CompressFormat::Lz4) => "Lz4",
            Some(CompressFormat::Tar) => "Tar",
            None => "",
        };
        let level = match job.level {
            Some(Level::Fastest) => "Fastest",
            Some(Level::Faster) => "Faster",
            Some(Level::Default) => "Default",
            Some(Level::Better) => "Better",
            Some(Level::Best) => "Best",
            None => "",
        };
        s.push_str(&format!(
            "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\"",
            job.id,
            job.source.display(),
            job.target.display()
        ));
        if !comp.is_empty() {
            s.push_str(&format!(",\n    compression: \"{comp}\""));
        }
        if !level.is_empty() {
            s.push_str(&format!(",\n    level: \"{level}\""));
        }
        if let Some(ignore) = &job.ignore {
            s.push_str(&format!(",\n    ignore: {ignore:?}"));
        }
        s.push_str("\n}");
    }
    s.push(']');
    s
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
        let mut msg = String::new();
        ids.into_iter().for_each(|id| match app.remove_job(id) {
            Some(_) => msg.push_str(&format!("Job with id {id} deleted successfully.\n")),
            None => msg.push_str(&format!(
                "Job deletion failed. Job with id {id} cannot be found.\n"
            )),
        });
        app.write()?;
        msg.remove(msg.len() - 1);
        println!("{}", msg);
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
/// * `compression` - Optional new compression format. If provided, replaces the job's compression.
/// * `level` - Optional new compression level. If provided, replaces the job's compression level.
/// * `ignore` - Optional new ignore list. If provided, replaces the job's ignore list.
/// * `clear` - Optional list of fields to clear (compression, level, ignore).
///
/// # Errors
/// Returns an error if the job is not found or the new path is invalid.
pub(crate) fn edit(params: EditParams) -> Result<()> {
    let EditParams {
        id,
        source,
        target,
        compression,
        level,
        ignore,
        clear,
    } = params;
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

        // Handle clear operations first
        if let Some(clear_fields) = &clear {
            for field in clear_fields {
                match field {
                    ClearField::Compression => {
                        job.compression = None;
                        job.level = None; // Clear level when clearing compression
                    }
                    ClearField::Level => {
                        job.level = None;
                    }
                    ClearField::Ignore => {
                        job.ignore = None;
                    }
                }
            }
        }

        // Handle set operations
        if let Some(comp) = compression {
            job.compression = Some(comp);
        }

        if let Some(lvl) = level {
            if job.compression.is_none() {
                eprintln!(
                    "The compression format is not set, and the compression level cannot be updated."
                );
                process::exit(1);
            }
            job.level = Some(lvl);
        }

        if let Some(ign) = ignore {
            job.ignore = Some(ign);
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
pub(crate) fn backup_config_file() {
    let config_file = application::config_file();
    let backed_config_file = application::backed_config_file();
    // If the configuration file does not exist, initialize it
    if !config_file.exists() {
        let app = Application::new();
        if let Err(e) = app.write() {
            eprintln!("Failed to initialize configuration file: {e}");
            process::exit(1);
        }
    }
    match fs::copy(config_file, backed_config_file) {
        Ok(_) => println!("Backup successfully!"),
        Err(e) => {
            eprintln!("Failed to backup configuration file: {e}");
            process::exit(1);
        }
    }
}

/// Reset the configuration file and back up the file before resetting.
pub(crate) fn reset_config_file() {
    let config_file = application::config_file();
    let backed_config_file = application::backed_config_file();
    // Backup the config file if it exists
    if config_file.exists() {
        if let Err(e) = fs::copy(config_file, backed_config_file) {
            eprintln!("Failed to backup configuration file: {e}");
            process::exit(1);
        }
    }
    // Initialize or reset the config file
    match Application::new().write() {
        Ok(_) => println!("Configuration file reset successfully!"),
        Err(e) => {
            eprintln!("Failed to reset configuration file: {e}");
            process::exit(1);
        }
    }
}

/// Rollback the last backed up configuration file.
pub(crate) fn rollback_config_file() {
    let backed_config_file = application::backed_config_file();
    if !backed_config_file.exists() {
        eprintln!("The backup configuration file does not exist.");
        process::exit(1);
    }
    let app = application::read_backed_config_file();
    match app.write() {
        Ok(_) => println!("Configuration file rolled back successfully."),
        Err(e) => {
            eprintln!("Failed to rollback configuration file: {e}");
            process::exit(1);
        }
    }
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
fn get_jobs(
    source: &Path,
    target: &Path,
    ignore: &Option<Vec<String>>,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let prefix = source.parent().unwrap_or(Path::new(""));
    let mut vec = vec![];
    let ignore_paths: Vec<PathBuf> = ignore
        .as_ref()
        .map(|dirs| dirs.iter().map(|s| source.join(s)).collect())
        .unwrap_or_default();

    for entry in WalkDir::new(source) {
        let entry = entry?;
        let path = entry.path();
        if ignore_paths.iter().any(|p| path.starts_with(p)) {
            continue;
        }

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
    let target_file = if target.exists() && target.is_dir() {
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

/// Asynchronously copy file from source to target, creating parent directories if needed.
async fn copy_file_async(source: PathBuf, target: PathBuf) -> Result<()> {
    let target_file = if target.exists() && target.is_dir() {
        let file_name = source.file_name().with_context(|| "Invalid file name")?;
        target.join(file_name)
    } else {
        target
    };

    if let Some(parent) = target_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    tokio::fs::copy(source, &target_file).await?;
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

#[cfg(test)]
mod tests {
    use crate::application::{CompressFormat, Job, Level};
    use crate::commands::display_jobs;
    use std::path::PathBuf;

    #[test]
    fn test_job_list_display() {
        let jobs = vec![
            Job {
                id: 1,
                source: PathBuf::from("/test/source1"),
                target: PathBuf::from("/test/target1"),
                compression: Some(CompressFormat::Zip),
                level: Some(Level::Fastest),
                ignore: None,
            },
            Job {
                id: 2,
                source: PathBuf::from("/test/source2"),
                target: PathBuf::from("/test/target2"),
                compression: Some(CompressFormat::Zstd),
                level: Some(Level::Best),
                ignore: Some(vec!["*.tmp".to_string()]),
            },
        ];

        let display_str = display_jobs(jobs);

        assert!(display_str.starts_with('['));
        assert!(display_str.ends_with(']'));
        assert!(display_str.contains("id: 1"));
        assert!(display_str.contains("id: 2"));
        assert!(display_str.contains("Zip"));
        assert!(display_str.contains("Zstd"));
    }
}
