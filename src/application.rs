//! Global configuration for this application.
//!
//! This module defines the core data structures and logic for managing
//! hbackup's persistent configuration, including backup jobs, compression formats,
//! and config file management. It provides serialization/deserialization for TOML and JSON,
//! and utilities for reading, writing, and migrating configuration files.

use crate::{Result, common::CONFIG_BACKUP_NAME, common::CONFIG_NAME, sysexits};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::{fs, io, process};

/// The main application configuration.
/// Stores the version and all backup jobs.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Application {
    /// Configuration file version.
    pub version: String,
    /// List of backup jobs.
    pub jobs: Vec<Job>,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            jobs: Vec::new(),
        }
    }
}

/// Represents a single backup job with a unique id, source, target, and optional compression.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Job {
    /// Unique job id.
    pub id: u32,
    /// Source file or directory path.
    pub source: PathBuf,
    /// Target file or directory path.
    pub target: PathBuf,
    /// Optional compression format for this job.
    pub compression: Option<CompressFormat>,
    /// Optional compression level for this job.
    pub level: Option<Level>,
    /// Optional ignore list
    pub ignore: Option<Vec<String>>,
}

/// Supported compression formats for backup jobs.
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug)]
pub(crate) enum CompressFormat {
    Gzip,
    Zip,
    Sevenz,
    Zstd,
    Bzip2,
    Xz,
    Lz4,
    Tar,
}

/// Supported compression level for backup jobs
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug)]
pub(crate) enum Level {
    Fastest,
    Faster,
    Default,
    Better,
    Best,
}

impl Application {
    /// Creates a new, empty application configuration.
    pub(crate) fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            jobs: vec![],
        }
    }

    /// Loads configuration from the config file, or returns a new config if not found.
    ///
    /// If the config file cannot be read, prints an error and exits.
    pub(crate) fn load_config() -> Self {
        if config_file_exists() {
            read_config_file()
        } else {
            Self::new()
        }
    }

    /// Adds a new backup job with a unique id.
    ///
    /// The id is automatically assigned to avoid conflicts.
    pub(crate) fn add_job(
        &mut self,
        source: PathBuf,
        target: PathBuf,
        compression: Option<CompressFormat>,
        level: Option<Level>,
        ignore: Option<Vec<String>>,
    ) {
        let id = if self.jobs.is_empty() {
            1
        } else {
            let job_ids: HashSet<u32> = self.jobs.iter().map(|j| j.id).collect();
            (1..u32::MAX)
                .find(|id| !job_ids.contains(id))
                .unwrap_or_else(|| {
                    eprintln!(
                        "The maximum number of jobs created is {}. No more jobs can be added.",
                        u32::MAX
                    );
                    process::exit(sysexits::EX_SOFTWARE);
                })
        };
        self.jobs.push(Job {
            id,
            source,
            target,
            compression,
            level,
            ignore,
        });
    }

    /// Removes all jobs from the configuration.
    pub(crate) fn reset_jobs(&mut self) {
        self.jobs = vec![];
    }

    /// Writes the current configuration to the config file.
    pub(crate) fn write(&self) -> Result<()> {
        write_config(self)?;
        Ok(())
    }

    /// Returns all jobs from the current configuration.
    pub(crate) fn get_jobs() -> Vec<Job> {
        Self::load_config().jobs
    }

    /// Removes a job by id. Returns Some if removed, None if not found.
    pub(crate) fn remove_job(&mut self, id: u32) -> Option<()> {
        if let Some(index) = self.jobs.iter().position(|j| j.id == id) {
            self.jobs.remove(index);
            Some(())
        } else {
            None
        }
    }
}

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

/// Returns the absolute path to the configuration file.
pub(crate) fn config_file() -> PathBuf {
    config_dir().join(CONFIG_NAME)
}

/// Returns the absolute path to the backup configuration file.
pub(crate) fn backed_config_file() -> PathBuf {
    config_dir().join(CONFIG_BACKUP_NAME)
}

/// Returns the configuration directory for the application, platform-specific.
#[cfg(not(target_os = "macos"))]
fn config_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        eprintln!("Couldn't get the home directory!!!");
        process::exit(sysexits::EX_UNAVAILABLE);
    });
    config_dir.join(PKG_NAME)
}

/// Returns the configuration directory for the application, platform-specific.
#[cfg(target_os = "macos")]
fn config_dir() -> PathBuf {
    let home_dir = match dirs::home_dir() {
        Some(home_dir) => home_dir,
        None => {
            eprintln!("Couldn't get the home directory!!!");
            process::exit(sysexits::EX_UNAVAILABLE);
        }
    };
    home_dir.join(".config").join(PKG_NAME)
}

/// Checks if the configuration file exists.
fn config_file_exists() -> bool {
    config_file().exists()
}

/// Writes the application configuration to the config file in TOML format.
///
/// Creates the parent directory if it does not exist.
pub(crate) fn write_config(data: &Application) -> Result<()> {
    let file_path = config_file();
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(file_path)?;
    let mut writer = io::BufWriter::new(file);
    let toml_str = toml::to_string_pretty(&data)?;
    writer.write_all(toml_str.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// Reads the default configuration file in TOML format.
fn read_config_file() -> Application {
    let file_path = config_file();
    let toml_str = match fs::read_to_string(&file_path) {
        Ok(toml_str) => toml_str,
        Err(err) => {
            eprintln!("Error reading config file: {err}");
            std::process::exit(1);
        }
    };
    match toml::from_str(&toml_str) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Error parsing config file: {err}");
            std::process::exit(1);
        }
    }
}

/// Reads the backup configuration file in TOML format.
pub(crate) fn read_backed_config_file() -> Application {
    let file_path = backed_config_file();
    let toml_str = match fs::read_to_string(&file_path) {
        Ok(toml_str) => toml_str,
        Err(err) => {
            eprintln!("Error reading backup config file: {err}");
            process::exit(sysexits::EX_IOERR);
        }
    };
    match toml::from_str(&toml_str) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Error parsing backup config file: {err}");
            process::exit(sysexits::EX_IOERR);
        }
    }
}

/// Initializes the configuration file for the application if it does not exist.
///
/// This function checks for the existence of both the new and old configuration files.
/// - If neither exists, it creates a new default configuration file in TOML format,
///   ensuring the parent directory exists.
/// - If only the old configuration file exists, it migrates the old configuration to the new location and format.
///
/// This ensures that the application always has a valid configuration file to work with.
pub(crate) fn init_config() {
    let old_config_file = old_config_file();
    let config_file = config_file();
    if !config_file.exists() && !old_config_file.exists() {
        let app = Application::new();

        let parent = config_file.parent().unwrap();
        fs::create_dir_all(parent).unwrap();

        let file = fs::File::create(config_file).unwrap();
        let mut writer = io::BufWriter::new(file);
        let toml_str = toml::to_string_pretty(&app).unwrap();
        writer.write_all(toml_str.as_bytes()).unwrap();
        writer.flush().unwrap();
    } else if !config_file.exists() && old_config_file.exists() {
        let app = read_old_config_file().unwrap();
        let toml_str = toml::to_string_pretty(&app).unwrap();
        let file = fs::File::create(config_file).unwrap();
        let mut writer = io::BufWriter::new(file);
        writer.write_all(toml_str.as_bytes()).unwrap();
        writer.flush().unwrap();
    }
}

/// Reads the old configuration file in JSON format and converts it to Application.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
fn read_old_config_file() -> Result<Application> {
    let file_path = old_config_file();
    let file = fs::File::open(&file_path)?;
    let reader = io::BufReader::new(&file);
    let app: Application = serde_json::from_reader(reader)?;
    Ok(app)
}

/// Returns the path to the old JSON configuration file.
fn old_config_file() -> PathBuf {
    config_dir().join(format!("{PKG_NAME}.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_config_file() {
        let file = config_dir().join("hbackup").join("config.toml");
        assert_eq!(config_file(), file);
    }

    #[test]
    fn test_backed_config_file() {
        let file = config_dir().join("hbackup").join("config_backup.toml");
        assert_eq!(backed_config_file(), file);
    }

    #[test]
    fn test_application_new() {
        let app = Application::new();
        assert_eq!(app.version, "1.0");
        assert!(app.jobs.is_empty());
    }

    #[test]
    fn test_application_add_job() {
        let mut app = Application::new();
        let source = PathBuf::from("/test/source");
        let target = PathBuf::from("/test/target");

        app.add_job(
            source.clone(),
            target.clone(),
            Some(CompressFormat::Gzip),
            Some(Level::Default),
            None,
        );

        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.jobs[0].id, 1);
        assert_eq!(app.jobs[0].source, source);
        assert_eq!(app.jobs[0].target, target);
        assert!(matches!(
            app.jobs[0].compression,
            Some(CompressFormat::Gzip)
        ));
        assert!(matches!(app.jobs[0].level, Some(Level::Default)));
    }

    #[test]
    fn test_application_add_multiple_jobs() {
        let mut app = Application::new();

        // Add first job
        app.add_job(
            PathBuf::from("/test/source1"),
            PathBuf::from("/test/target1"),
            Some(CompressFormat::Zip),
            Some(Level::Fastest),
            None,
        );

        // Add second job
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            Some(CompressFormat::Zstd),
            Some(Level::Best),
            Some(vec!["*.log".to_string()]),
        );

        assert_eq!(app.jobs.len(), 2);
        assert_eq!(app.jobs[0].id, 1);
        assert_eq!(app.jobs[1].id, 2);
        assert_ne!(app.jobs[0].id, app.jobs[1].id);
    }

    #[test]
    fn test_application_remove_job() {
        let mut app = Application::new();

        // Add jobs
        app.add_job(
            PathBuf::from("/test/source1"),
            PathBuf::from("/test/target1"),
            None,
            None,
            None,
        );
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            None,
            None,
            None,
        );

        assert_eq!(app.jobs.len(), 2);

        // Remove first job
        let result = app.remove_job(1);
        assert!(result.is_some());
        assert_eq!(app.jobs.len(), 1);
        assert_eq!(app.jobs[0].id, 2);

        // Try to remove non-existent job
        let result = app.remove_job(999);
        assert!(result.is_none());
        assert_eq!(app.jobs.len(), 1);
    }

    #[test]
    fn test_application_reset_jobs() {
        let mut app = Application::new();

        // Add some jobs
        app.add_job(
            PathBuf::from("/test/source1"),
            PathBuf::from("/test/target1"),
            None,
            None,
            None,
        );
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            None,
            None,
            None,
        );

        assert_eq!(app.jobs.len(), 2);

        app.reset_jobs();
        assert!(app.jobs.is_empty());
    }

    #[test]
    fn test_application_serialization() {
        let mut app = Application::new();
        app.add_job(
            PathBuf::from("/test/source"),
            PathBuf::from("/test/target"),
            Some(CompressFormat::Gzip),
            Some(Level::Default),
            Some(vec!["*.log".to_string()]),
        );

        // Test TOML serialization
        let toml_str = toml::to_string(&app).expect("Failed to serialize to TOML");
        assert!(toml_str.contains("version = \"1.0\""));
        assert!(toml_str.contains("id = 1"));
        assert!(toml_str.contains("Gzip"));

        // Test TOML deserialization
        let deserialized: Application =
            toml::from_str(&toml_str).expect("Failed to deserialize from TOML");
        assert_eq!(deserialized.version, app.version);
        assert_eq!(deserialized.jobs.len(), app.jobs.len());
        assert_eq!(deserialized.jobs[0].id, app.jobs[0].id);
        assert_eq!(deserialized.jobs[0].source, app.jobs[0].source);
        assert_eq!(deserialized.jobs[0].target, app.jobs[0].target);
    }

    #[test]
    fn test_application_default() {
        let app = Application::default();
        assert_eq!(app.version, "1.0");
        assert!(app.jobs.is_empty());
    }

    /// Returns the configuration directory for testing, platform-specific.
    fn config_dir() -> PathBuf {
        if cfg!(target_os = "macos") {
            let home = env::var("HOME").unwrap();
            PathBuf::from(home).join(".config")
        } else {
            dirs::config_dir().unwrap()
        }
    }
}
