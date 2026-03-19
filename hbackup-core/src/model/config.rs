use crate::error::{HbackupError, Result};
use crate::model::job::Job;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, vec};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    version: String,
    jobs: Vec<Job>,
}

impl Config {
    /// Adds a new backup job to the configuration.
    pub fn add_job(&mut self, mut new_job: Job) -> Result<Job> {
        new_job.source = fs::canonicalize(&new_job.source)?;
        new_job.target = fs::canonicalize(&new_job.target)?;

        if new_job.id == 0 {
            let max_id = self.jobs.iter().map(|j| j.id).max().unwrap_or(0);
            new_job.id = max_id + 1;
        } else {
            if self.jobs.iter().any(|j| j.id == new_job.id) {
                return Err(HbackupError::RuntimeError(format!(
                    "Job ID {} is already taken",
                    new_job.id
                )));
            }
        }
        self.jobs.push(new_job);

        Ok(self.jobs.last().unwrap().clone())
    }

    pub fn delete(&mut self, ids: Vec<u32>) -> Result<Vec<Job>> {
        let ids_set: HashSet<u32> = ids.into_iter().collect();
        for id in ids_set.iter() {
            if !self.jobs.iter().any(|j| j.id == *id) {
                return Err(HbackupError::JobNotFound(*id));
            }
        }

        let mut removed_jobs = Vec::new();
        self.jobs.retain(|job| {
            if ids_set.contains(&job.id) {
                removed_jobs.push(job.clone());
                false
            } else {
                true
            }
        });

        Ok(removed_jobs)
    }

    pub fn reset_jobs(&mut self) {
        self.jobs = vec![];
    }

    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    pub fn list_by_ids(&self, ids: &[u32]) -> Vec<&Job> {
        self.jobs
            .iter()
            .filter(|job| ids.contains(&job.id))
            .collect()
    }

    pub fn list_by_gte(&self, id: u32) -> Vec<&Job> {
        self.jobs.iter().filter(|job| job.id >= id).collect()
    }

    pub fn list_by_lte(&self, id: u32) -> Vec<&Job> {
        self.jobs.iter().filter(|job| job.id <= id).collect()
    }

    pub fn get_job(&self, id: u32) -> Option<&Job> {
        self.jobs.iter().find(|j| j.id == id)
    }

    pub fn get_job_mut(&mut self, id: u32) -> Option<&mut Job> {
        self.jobs.iter_mut().find(|j| j.id == id)
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    version: String,
}

impl ConfigManager {
    /// Initialize ConfigManager using the platform-specific base directory.
    pub fn new(app_name: &str, config_name: &str, version: String) -> Result<Self> {
        let base_dir = get_base_config_dir()?;
        let config_path = base_dir.join(app_name).join(config_name);
        Ok(Self {
            config_path,
            version,
        })
    }

    pub fn from_path_and_version(config_path: PathBuf, version: String) -> Self {
        Self {
            config_path,
            version,
        }
    }

    pub fn config_path(&self) -> &Path {
        self.config_path.as_path()
    }

    fn ensure_dir(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    /// Writes the application configuration to the config file in TOML format.
    ///
    /// This method ensures the parent directory exists before writing.
    pub fn save(&self, config: &Config) -> Result<()> {
        self.ensure_dir()?;

        let toml_str = toml::to_string_pretty(config)?;
        fs::write(&self.config_path, toml_str)?;

        Ok(())
    }

    /// Loads the configuration from disk with automatic schema migration.
    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            let default_config = self.default_config();
            self.save(&default_config)?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&self.config_path)?;

        // If parsing fails, the file might be corrupted or severely outdated
        let mut config: Config = match toml::from_str(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                // Disaster recovery: backup the broken file and reset to default
                let mut corrupted_path = self.config_path.clone();
                corrupted_path.set_extension("toml.corrupted");
                eprintln!("\n⚠️  [Warning] Configuration file is corrupted or incompatible!");
                eprintln!("   Warning: {}", e);
                if fs::copy(&self.config_path, &corrupted_path).is_ok() {
                    eprintln!(
                        "   Your broken config has been backed up to: {:?}",
                        corrupted_path
                    );
                }

                let default_config = self.default_config();
                self.save(&default_config)?;
                println!("   Notice: A new default configuration has been initialized.\n");
                return Ok(default_config);
            }
        };

        if config.version != self.version {
            self.backup()?;
            config.version = self.version.clone();
            self.save(&config)?;
        }

        Ok(config)
    }

    fn default_config(&self) -> Config {
        Config {
            version: self.version.clone(),
            jobs: vec![],
        }
    }

    /// Backs up the current configuration file to 'config.toml.bak'.
    pub fn backup(&self) -> Result<()> {
        if !self.config_path.exists() {
            return Err(HbackupError::RuntimeError(format!(
                "Backup failed: Config file {:?} does not exist",
                self.config_path
            )));
        }

        let mut backup_path = self.config_path.clone();
        let mut file_name = self
            .config_path
            .file_name()
            .ok_or_else(|| HbackupError::RuntimeError("Invalid config path".into()))?
            .to_os_string();
        file_name.push(".bak");
        backup_path.set_file_name(file_name);

        fs::copy(&self.config_path, backup_path)?;
        Ok(())
    }
}

/// Returns the configuration directory for the application, platform-specific.
#[cfg(not(target_os = "macos"))]
fn get_base_config_dir() -> Result<PathBuf> {
    dirs::config_dir().ok_or_else(|| {
        HbackupError::EnvironmentUnavailable("Could not get the config directory".into())
    })
}

/// Returns the configuration directory for the application, platform-specific.
/// On macOS, we specifically use ~/.config as per your requirement.
#[cfg(target_os = "macos")]
fn get_base_config_dir() -> Result<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config")).ok_or_else(|| {
        HbackupError::EnvironmentUnavailable("Could not get the home directory".into())
    })
}
