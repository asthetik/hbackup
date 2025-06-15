//! Global configuration for this application.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;
use std::{fmt, fs, io, process};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Global configuration for this application.
/// Stores all backup jobs.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Application {
    pub jobs: Vec<Job>,
}

/// Represents a single backup job with a unique id, source, and target path.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    /// Unique job id.
    pub id: u32,
    /// Source file path.
    pub source: PathBuf,
    /// Target file or directory path.
    pub target: PathBuf,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\n    id: {},\n    source: \"{}\",\n    target: \"{}\",\n}}",
            self.id,
            self.source.display(),
            self.target.display()
        )
    }
}

///  A wrapper for displaying a list of jobs in a formatted way.
pub struct JobList(pub Vec<Job>);

impl fmt::Display for JobList {
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
        Self { jobs: vec![] }
    }

    /// Loads configuration from the config file, or returns a new config if not found.
    pub fn load_config() -> Self {
        if config_file_exists() {
            read_config_file().expect("Failed to read configuration file.")
        } else {
            Application::new()
        }
    }

    /// Adds a new backup job with a unique id.
    pub fn add_job(&mut self, source: PathBuf, target: PathBuf) {
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
                    eprintln!(
                        "The maximum number of jobs created is {}. No more jobs can be added.",
                        u32::MAX
                    );
                    process::exit(1);
                });
            self.jobs.push(Job { id, source, target });
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
    config_dir().join(format!("{PKG_NAME}.json"))
}

pub fn backup_config_file() -> PathBuf {
    config_dir().join(format!("{PKG_NAME}_backup.json"))
}

fn config_dir() -> PathBuf {
    let config_dir = if cfg!(target_os = "macos") {
        let home_dir = match dirs::home_dir() {
            Some(home_dir) => home_dir,
            None => {
                eprintln!("Couldn't get the home directory!!!");
                process::exit(1);
            }
        };
        home_dir.join(".config")
    } else {
        match dirs::config_dir() {
            Some(home_dir) => home_dir,
            None => {
                eprintln!("Couldn't get the home directory!!!");
                process::exit(1);
            }
        }
    };
    config_dir.join(PKG_NAME)
}

/// Checks if the configuration file exists.
fn config_file_exists() -> bool {
    config_file().exists()
}

/// Writes the application configuration to the config file.
pub fn write_config(data: &Application) -> Result<()> {
    let file_path = config_file();
    if !file_path.exists() {
        // The default configuration file path must exist in the parent folder
        let parent = file_path.parent().unwrap();
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(file_path)?;
    let writer = io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data)?;
    Ok(())
}

/// read the default configuration file.
fn read_config_file() -> Result<Application> {
    let file_path = config_file();
    let file = fs::File::open(&file_path)?;
    let reader = io::BufReader::new(&file);
    let app: Application = serde_json::from_reader(reader)?;
    Ok(app)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env;


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
