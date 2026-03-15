use crate::error::{HbackupError, Result};
use crate::model::job::Job;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CURRENT_VERSION: &str = "1.1";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub jobs: Vec<Job>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION.to_string(),
            jobs: vec![],
        }
    }
}

impl Config {
    /// Adds a new backup job to the configuration.
    pub fn add_job(&mut self, mut new_job: Job) -> Result<Job> {
        new_job.source = fs::canonicalize(&new_job.source)?;

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
}

pub struct ConfigManager {
    config_path: PathBuf,
}

impl ConfigManager {
    /// Initialize ConfigManager using the platform-specific base directory.
    pub fn new(app_name: &str, config_name: &str) -> Result<Self> {
        let base_dir = get_base_config_dir()?;
        let app_dir = base_dir.join(app_name);

        if !app_dir.exists() {
            fs::create_dir_all(&app_dir)?;
        }
        let config_path = app_dir.join(config_name);
        Ok(Self { config_path })
    }

    /// Writes the application configuration to the config file in TOML format.
    ///
    /// This method ensures the parent directory exists before writing.
    pub fn save(&self, config: &Config) -> Result<()> {
        // Ensure the parent directory exists
        if let Some(parent) = self.config_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(config)?;
        fs::write(&self.config_path, toml_str)?;

        Ok(())
    }

    /// Loads the configuration from disk with automatic schema migration.
    pub fn load(&self) -> Result<Config> {
        if !self.config_path.exists() {
            let default_config = Config::default();
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

                let default_config = Config::default();
                self.save(&default_config)?;
                println!("   Notice: A new default configuration has been initialized.\n");
                return Ok(default_config);
            }
        };

        if config.version != CURRENT_VERSION {
            self.backup()?;
            config.version = CURRENT_VERSION.to_string();
            self.save(&config)?;
        }

        Ok(config)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::job::Strategy;
    use tempfile::tempdir;

    #[test]
    fn config_default_has_empty_jobs_and_version() {
        let cfg = Config::default();
        assert_eq!(cfg.version, CURRENT_VERSION);
        assert!(cfg.jobs.is_empty());
    }

    #[test]
    fn add_job_sets_id_for_zero() {
        let mut cfg = Config::default();
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();

        let job = Job {
            id: 0,
            source: source.clone(),
            target: temp.path().join("target"),
            strategy: Strategy::Copy,
        };

        let saved = cfg.add_job(job).unwrap();
        assert_eq!(saved.id, 1);
        assert_eq!(cfg.jobs.len(), 1);
    }

    #[test]
    fn add_job_duplicate_id_fails() {
        let mut cfg = Config::default();
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir(&source).unwrap();

        let mut job = Job {
            id: 5,
            source: source.clone(),
            target: temp.path().join("target"),
            strategy: Strategy::Copy,
        };
        cfg.add_job(job.clone()).unwrap();

        job.source = source.clone();
        let err = cfg.add_job(job).unwrap_err();
        assert!(matches!(err, crate::error::HbackupError::RuntimeError(_)));
    }

    fn with_temp_config<F, R>(temp: &tempfile::TempDir, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let original_home = std::env::var_os("HOME");
        let original_xdg = std::env::var_os("XDG_CONFIG_HOME");

        if cfg!(target_os = "macos") {
            unsafe {
                std::env::set_var("HOME", temp.path());
            }
        } else {
            unsafe {
                std::env::set_var("XDG_CONFIG_HOME", temp.path());
            }
        }

        let result = f();

        if let Some(value) = original_home {
            unsafe {
                std::env::set_var("HOME", value);
            }
        } else {
            unsafe {
                std::env::remove_var("HOME");
            }
        }

        if let Some(value) = original_xdg {
            unsafe {
                std::env::set_var("XDG_CONFIG_HOME", value);
            }
        } else {
            unsafe {
                std::env::remove_var("XDG_CONFIG_HOME");
            }
        }

        result
    }

    #[test]
    fn config_manager_save_load_and_backup() {
        let temp = tempdir().unwrap();
        let manager = with_temp_config(&temp, || {
            ConfigManager::new("hbackup_test", "config.toml").unwrap()
        });

        let cfg = Config::default();
        manager.save(&cfg).unwrap();

        let loaded = manager.load().unwrap();
        assert_eq!(loaded.version, CURRENT_VERSION);

        // set old version to force backup during load
        let mut old_cfg = cfg;
        old_cfg.version = "0.0".to_string();
        manager.save(&old_cfg).unwrap();

        let upgraded = manager.load().unwrap();
        assert_eq!(upgraded.version, CURRENT_VERSION);
        assert!(manager.config_path.with_extension("toml.bak").exists());
    }

    #[test]
    fn config_manager_load_corrupt_recreates() {
        let temp = tempdir().unwrap();

        let corrupted_path = with_temp_config(&temp, || {
            let manager = ConfigManager::new("hbackup_test", "config.toml").unwrap();
            std::fs::write(&manager.config_path, "NOT TOML").unwrap();
            let loaded = manager.load().unwrap();
            assert_eq!(loaded.version, CURRENT_VERSION);
            manager.config_path.with_extension("toml.corrupted")
        });

        assert!(corrupted_path.exists());
    }

    #[test]
    fn config_manager_backup_missing_file_error() {
        let temp = tempdir().unwrap();

        with_temp_config(&temp, || {
            let missing_manager = ConfigManager::new("hbackup_test", "missing.toml").unwrap();
            let err = missing_manager.backup().unwrap_err();
            assert!(matches!(err, crate::error::HbackupError::RuntimeError(_)));
        });
    }
}
