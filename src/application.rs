//! Global configuration for this application.
//!
//! This module defines the core data structures and logic for managing
//! hbackup's persistent configuration, including backup jobs, compression formats,
//! and config file management. It provides serialization/deserialization for TOML and JSON,
//! and utilities for reading, writing, and migrating configuration files.

use crate::{Result, constants::CONFIG_BACKUP_NAME, constants::CONFIG_NAME, sysexits};
use hbackup::job::{BackupModel, CompressFormat, Job, Level};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
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
            jobs: vec![],
        }
    }
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
        model: Option<BackupModel>,
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
            model,
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

    pub(crate) fn list_by_ids(ids: Vec<u32>) -> Vec<Job> {
        Self::get_jobs()
            .into_iter()
            .filter(|job| ids.contains(&job.id))
            .collect()
    }

    /// Lists backup jobs by their IDs.
    pub(crate) fn list_by_gte(id: u32) -> Vec<Job> {
        Self::get_jobs()
            .into_iter()
            .filter(|job| job.id >= id)
            .collect()
    }

    pub(crate) fn list_by_lte(id: u32) -> Vec<Job> {
        Self::get_jobs()
            .into_iter()
            .filter(|job| job.id <= id)
            .collect()
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

/// Returns the absolute path to the configuration file.
pub(crate) fn config_file() -> PathBuf {
    config_dir().join(CONFIG_NAME)
}

/// Returns the absolute path to the backup configuration file.
fn backed_config_file() -> PathBuf {
    config_dir().join(CONFIG_BACKUP_NAME)
}

/// Returns the configuration directory for the application, platform-specific.
#[cfg(not(target_os = "macos"))]
fn config_dir() -> PathBuf {
    use crate::constants::PKG_NAME;

    let config_dir = dirs::config_dir().unwrap_or_else(|| {
        eprintln!("Couldn't get the home directory!!!");
        process::exit(sysexits::EX_UNAVAILABLE);
    });
    config_dir.join(PKG_NAME)
}

/// Returns the configuration directory for the application, platform-specific.
#[cfg(target_os = "macos")]
fn config_dir() -> PathBuf {
    use crate::constants::PKG_NAME;

    let home_dir = dirs::home_dir().unwrap_or_else(|| {
        eprintln!("Couldn't get the home directory!!!");
        process::exit(sysexits::EX_UNAVAILABLE);
    });
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
    let toml_str = fs::read_to_string(&file_path).unwrap_or_else(|e| {
        eprintln!("Error reading config file: {e}");
        std::process::exit(1);
    });
    toml::from_str(&toml_str).unwrap_or_else(|e| {
        eprintln!("Error parsing config file: {e}");
        std::process::exit(1);
    })
}

/// Reads the backup configuration file in TOML format.
pub(crate) fn read_backed_config_file() -> Application {
    let file_path = backed_config_file();
    let toml_str = fs::read_to_string(&file_path).unwrap_or_else(|e| {
        eprintln!("Error reading backup config file: {e}");
        process::exit(sysexits::EX_IOERR);
    });
    toml::from_str(&toml_str).unwrap_or_else(|e| {
        eprintln!("Error parsing backup config file: {e}");
        process::exit(sysexits::EX_IOERR);
    })
}

/// Initializes the configuration file for the application if it does not exist.
/// This ensures that the application always has a valid configuration file to work with.
pub(crate) fn init_config() {
    let config_file = config_file();
    if !config_file.exists() {
        let app = Application::new();

        let parent = config_file.parent().unwrap_or_else(|| Path::new(""));
        fs::create_dir_all(parent).unwrap();

        let file = fs::File::create(config_file).unwrap();
        let mut writer = io::BufWriter::new(file);
        let toml_str = toml::to_string_pretty(&app).unwrap();
        writer.write_all(toml_str.as_bytes()).unwrap();
        writer.flush().unwrap();
    }
}

/// Back up the configuration file to a backup location.
pub(crate) fn backup_config_file() {
    let config_file = config_file();
    let backed_config_file = backed_config_file();
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
    let config_file = config_file();
    let backed_config_file = backed_config_file();
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
    let backed_config_file = backed_config_file();
    if !backed_config_file.exists() {
        eprintln!("The backup configuration file does not exist.");
        process::exit(1);
    }
    let app = read_backed_config_file();
    match app.write() {
        Ok(_) => println!("Configuration file rolled back successfully."),
        Err(e) => {
            eprintln!("Failed to rollback configuration file: {e}");
            process::exit(1);
        }
    }
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
            None,
        );

        // Add second job
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            Some(CompressFormat::Zstd),
            Some(Level::Best),
            Some(vec!["*.log".to_string()]),
            None,
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
            None,
        );
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            None,
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
            None,
        );
        app.add_job(
            PathBuf::from("/test/source2"),
            PathBuf::from("/test/target2"),
            None,
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
            None,
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
