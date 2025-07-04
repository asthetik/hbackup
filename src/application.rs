//! Global configuration for this application.
//!
//! This module defines the core data structures and logic for managing
//! hbackup's persistent configuration, including backup jobs, compression formats,
//! and config file management. It provides serialization/deserialization for TOML and JSON,
//! and utilities for reading, writing, and migrating configuration files.

use crate::{CONFIG_BACKUP_NAME, CONFIG_NAME, Result, sysexits};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::{fmt, fs, io, process};

/// The main application configuration.
/// Stores the version and all backup jobs.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Application {
    /// Configuration file version.
    #[serde(default = "default_version")]
    pub version: String,
    /// List of backup jobs.
    pub jobs: Vec<Job>,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// Represents a single backup job with a unique id, source, target, and optional compression.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    /// Unique job id.
    pub id: u32,
    /// Source file or directory path.
    pub source: PathBuf,
    /// Target file or directory path.
    pub target: PathBuf,
    /// Optional compression format for this job.
    pub compression: Option<CompressFormat>,
}

impl fmt::Display for Job {
    /// Pretty-print a job, including compression if present.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let comp = match self.compression {
            Some(CompressFormat::Gzip) => "Gzip",
            Some(CompressFormat::Zip) => "Zip",
            Some(CompressFormat::Sevenz) => "Sevenz",
            Some(CompressFormat::Zstd) => "Zstd",
            Some(CompressFormat::Bzip2) => "Bzip2",
            Some(CompressFormat::Xz) => "Xz",
            None => "",
        };
        if comp.is_empty() {
            write!(
                f,
                "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\"\n}}",
                self.id,
                self.source.display(),
                self.target.display(),
            )
        } else {
            write!(
                f,
                "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\",\n    compression: \"{}\"\n}}",
                self.id,
                self.source.display(),
                self.target.display(),
                comp
            )
        }
    }
}

/// Supported compression formats for backup jobs.
#[derive(ValueEnum, Serialize, Deserialize, Clone, Debug)]
pub enum CompressFormat {
    Gzip,
    Zip,
    Sevenz,
    Zstd,
    Bzip2,
    Xz,
}

/// A wrapper for displaying a list of jobs in a formatted way.
pub struct JobList(pub Vec<Job>);

impl fmt::Display for JobList {
    /// Pretty-print the job list as an array.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, job) in self.0.iter().enumerate() {
            write!(f, "{job}")?;
            if i != self.0.len() - 1 {
                writeln!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl Application {
    /// Creates a new, empty application configuration.
    pub fn new() -> Self {
        Application {
            version: default_version(),
            jobs: vec![],
        }
    }

    /// Loads configuration from the config file, or returns a new config if not found.
    ///
    /// If the config file cannot be read, prints an error and exits.
    pub fn load_config() -> Self {
        if config_file_exists() {
            match read_config_file() {
                Ok(app) => app,
                Err(e) => {
                    eprintln!("Failed to read configuration file\n{e}");
                    process::exit(sysexits::EX_CONFIG);
                }
            }
        } else {
            Application::new()
        }
    }

    /// Adds a new backup job with a unique id.
    ///
    /// The id is automatically assigned to avoid conflicts.
    pub fn add_job(
        &mut self,
        source: PathBuf,
        target: PathBuf,
        compression: Option<CompressFormat>,
    ) {
        if self.jobs.is_empty() {
            self.jobs.push(Job {
                id: 1,
                source,
                target,
                compression,
            });
        } else {
            let job_ids: HashSet<u32> = self.jobs.iter().map(|j| j.id).collect();
            let id = (1..u32::MAX)
                .find(|id| !job_ids.contains(id))
                .unwrap_or_else(|| {
                    eprintln!(
                        "The maximum number of jobs created is {}. No more jobs can be added.",
                        u32::MAX
                    );
                    process::exit(sysexits::EX_SOFTWARE);
                });
            self.jobs.push(Job {
                id,
                source,
                target,
                compression,
            });
        }
    }

    /// Removes all jobs from the configuration.
    pub fn reset_jobs(&mut self) {
        self.jobs = vec![];
    }

    /// Writes the current configuration to the config file.
    pub fn write(&self) -> Result<()> {
        write_config(self)?;
        Ok(())
    }

    /// Returns all jobs from the current configuration.
    pub fn get_jobs() -> Vec<Job> {
        Application::load_config().jobs
    }

    /// Removes a job by id. Returns Some if removed, None if not found.
    pub fn remove_job(&mut self, id: u32) -> Option<()> {
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
pub fn config_file() -> PathBuf {
    config_dir().join(CONFIG_NAME)
}

/// Returns the absolute path to the backup configuration file.
pub fn backed_config_file() -> PathBuf {
    config_dir().join(CONFIG_BACKUP_NAME)
}

/// Returns the configuration directory for the application, platform-specific.
fn config_dir() -> PathBuf {
    let config_dir = if cfg!(target_os = "macos") {
        let home_dir = match dirs::home_dir() {
            Some(home_dir) => home_dir,
            None => {
                eprintln!("Couldn't get the home directory!!!");
                process::exit(sysexits::EX_UNAVAILABLE);
            }
        };
        home_dir.join(".config")
    } else {
        match dirs::config_dir() {
            Some(home_dir) => home_dir,
            None => {
                eprintln!("Couldn't get the home directory!!!");
                process::exit(sysexits::EX_UNAVAILABLE);
            }
        }
    };
    config_dir.join(PKG_NAME)
}

/// Checks if the configuration file exists.
fn config_file_exists() -> bool {
    config_file().exists()
}

/// Writes the application configuration to the config file in TOML format.
///
/// Creates the parent directory if it does not exist.
pub fn write_config(data: &Application) -> Result<()> {
    let file_path = config_file();
    if !file_path.exists() {
        // The default configuration file path must exist in the parent folder
        let parent = file_path.parent().unwrap();
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(file_path)?;
    let mut writer = io::BufWriter::new(file);
    let toml_str = toml::to_string_pretty(&data).unwrap();
    writer.write_all(toml_str.as_bytes()).unwrap();
    writer.flush().unwrap();
    Ok(())
}

/// Reads the default configuration file in TOML format.
fn read_config_file() -> Result<Application> {
    let file_path = config_file();
    let toml_str = fs::read_to_string(&file_path)?;
    let app = toml::from_str(&toml_str)?;
    Ok(app)
}

/// Reads the backup configuration file in TOML format.
pub fn read_backed_config_file() -> Result<Application> {
    let file_path = backed_config_file();
    let toml_str = fs::read_to_string(&file_path)?;
    let app = toml::from_str(&toml_str)?;
    Ok(app)
}

/// Initializes the configuration file for the application if it does not exist.
///
/// This function checks for the existence of both the new and old configuration files.
/// - If neither exists, it creates a new default configuration file in TOML format,
///   ensuring the parent directory exists.
/// - If only the old configuration file exists, it migrates the old configuration to the new location and format.
///
/// This ensures that the application always has a valid configuration file to work with.
pub fn init_config() {
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
mod test {
    use super::*;
    use std::env;

    #[test]
    fn test_config_file() {
        let file = config_dir().join("hbackup").join("config.toml");
        assert_eq!(config_file(), file);
    }

    /// Returns the configuration directory for testing, platform-specific.
    fn config_dir() -> PathBuf {
        if cfg!(target_os = "macos") {
            let home = env::var("HOME").unwrap();
            let mut home_dir = PathBuf::from(home);
            home_dir.push(".config");
            home_dir
        } else {
            dirs::config_dir().unwrap()
        }
    }
}
