use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::{fs, io};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Global configuration for this application
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Application {
    pub jobs: Vec<Job>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub id: u32,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Application {
    pub fn new() -> Self {
        Self { jobs: vec![] }
    }

    /// load all configuration data
    pub fn load_config() -> Self {
        if config_file_exists() {
            let file = read_config_file().expect("Failed to read config file");
            let reader = io::BufReader::new(&file);
            serde_json::from_reader(reader).expect("Failed to parse config file")
        } else {
            let app = Application::new();
            write_config(&app).expect("Failed to write config file");
            app
        }
    }

    /// add a job
    pub fn add_job(&mut self, source: PathBuf, target: PathBuf) -> Result<()> {
        if self.jobs.is_empty() {
            self.jobs.push(Job {
                id: 0,
                source,
                target,
            });
        } else {
            let job_ids: HashSet<u32> = self.jobs.iter().map(|j| j.id).collect();
            let id = (0..u32::MAX)
                .find(|id| !job_ids.contains(id))
                .unwrap_or_else(|| {
                    panic!(
                        "The maximum number of jobs created is {}. No more jobs can be added.",
                        u32::MAX
                    )
                });
            self.jobs.push(Job { id, source, target });
        }
        Ok(())
    }

    /// reset jobs
    pub fn reset_jobs(&mut self) {
        self.jobs = vec![];
    }

    /// write configuration data
    pub fn write(&self) -> Result<()> {
        write_config(self)?;
        Ok(())
    }

    pub fn get_jobs() -> Vec<Job> {
        Application::load_config().jobs
    }

    pub fn remove_job(&mut self, id: u32) -> Option<()> {
        if let Some(index) = self.jobs.iter().position(|j| j.id == id) {
            self.jobs.remove(index);
            Some(())
        } else {
            None
        }
    }
}

pub fn config_file() -> PathBuf {
    let mut file = if cfg!(target_os = "macos") {
        let mut home_dir = dirs::home_dir().unwrap();
        home_dir.push(".config");
        home_dir
    } else {
        dirs::config_dir().unwrap()
    };
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const FILE_NAME: &str = concat!(env!("CARGO_PKG_NAME"), ".json");
    file.push(PKG_NAME);
    file.push(FILE_NAME);
    file
}

fn config_file_exists() -> bool {
    config_file().exists()
}

/// Create a default configuration file and initialize configuration parameters
fn create_config_file() -> Result<File> {
    let file_path = config_file();
    // Reset configuration file even if it exists
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
    Ok(file)
}

/// Write supported configuration data
fn write_config(data: &Application) -> Result<()> {
    let file = create_config_file()?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data)?;
    Ok(())
}

/// read the default configuration file.
fn read_config_file() -> Result<File> {
    let file_path = config_file();
    let file = fs::File::open(&file_path)?;
    Ok(file)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;

    #[test]
    fn default() {
        let application = Application::default();
        assert_eq!(application.jobs.len(), 0);
    }

    #[test]
    fn test_create_config_file() {
        assert!(create_config_file().is_ok());
    }

    #[test]
    fn test_config_file() {
        let mut file = config_dir();
        const PKG_NAME: &str = env!("CARGO_PKG_NAME");
        const FILE_NAME: &str = concat!(env!("CARGO_PKG_NAME"), ".json");
        file.push(PKG_NAME);
        file.push(FILE_NAME);

        assert_eq!(config_file(), file);
    }

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
