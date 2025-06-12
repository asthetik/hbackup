use std::{
    collections::HashSet,
    env,
    error::Error,
    fs::{self},
    io,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

use crate::path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: u32,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Task {
    pub fn new(id: Option<u32>, source: PathBuf, target: PathBuf) -> Self {
        match id {
            Some(id) => Task { id, source, target },
            None => Task::new_id(source, target),
        }
    }

    pub fn new_id(source: PathBuf, target: PathBuf) -> Self {
        let config_file = config_file();
        if path::check_path(&config_file).is_err() {
            Task {
                id: 0,
                source,
                target,
            }
        } else {
            let tasks = Task::get_all().unwrap();
            let task_ids: HashSet<u32> = tasks.iter().map(|t| t.id).collect();

            let min_unused_id = (0..u32::MAX)
                .find(|id| !task_ids.contains(id))
                .expect("No available id found");

            Task {
                id: min_unused_id,
                source,
                target,
            }
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_file = config_file();
        if fs::metadata(&config_file).is_err() {
            self.write_task()?;
        } else {
            let mut tasks = Task::get_all()?;
            if tasks.iter().any(|task| task.id == self.id) {
                return Err("Unable to create a task with the same id".into());
            }
            tasks.push(self.clone());
            Task::write(&tasks)?;
        }
        println!("write successfully!");
        Ok(())
    }

    pub fn get_all() -> Result<Vec<Task>, Box<dyn Error>> {
        let path = config_file();
        if !path::file_exists(&path) {
            return Ok(vec![]);
        }
        let file = fs::File::open(path)?;
        let reader = io::BufReader::new(file);
        let tasks: Vec<Task> = serde_json::from_reader(reader)?;
        Ok(tasks)
    }

    pub fn get(id: &u32) -> Result<Task, Box<dyn Error>> {
        let task = Task::get_all()?.into_iter().find(|t| t.id == *id);
        let task = task.ok_or_else(|| "No matching tasks found.".to_string())?;
        Ok(task)
    }

    fn write_task(&self) -> Result<(), Box<dyn Error>> {
        let path = config_path();
        if fs::metadata(&path).is_err() {
            fs::create_dir_all(&path)?;
        }
        let file = config_file();
        let file = fs::File::create(&file)?;
        let writer = io::BufWriter::new(file);
        let tasks = vec![self];
        serde_json::to_writer_pretty(writer, &tasks)?;
        Ok(())
    }

    fn write(tasks: &[Task]) -> Result<(), Box<dyn Error>> {
        let path = config_file();
        let file = fs::File::create(path)?;
        let writer = io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, tasks)?;
        Ok(())
    }

    /// Deletes a task by its ID.
    pub fn delete_by_id(id: u32) -> Result<(), Box<dyn Error>> {
        let mut tasks = Task::get_all()?;
        if let Some(pos) = tasks.iter().position(|task| task.id == id) {
            tasks.remove(pos);
            Task::write(&tasks)?;
        } else {
            return Err("Task not found".into());
        }
        Ok(())
    }

    /// Deletes all tasks from the task file.
    pub fn delete_all() -> Result<(), Box<dyn Error>> {
        let path = config_file();
        if fs::metadata(&path).is_ok() {
            let tasks = vec![];
            Task::write(&tasks)?;
        }
        Ok(())
    }
}

pub fn config_file() -> PathBuf {
    let mut path = config_path();
    const FILE_NAME: &str = concat!(env!("CARGO_PKG_NAME"), ".json");
    path.push(FILE_NAME);
    path
}

fn config_path() -> PathBuf {
    let mut config_dir = if cfg!(target_os = "macos") {
        let mut home_dir = dirs::home_dir().unwrap();
        home_dir.push(".config");
        home_dir
    } else {
        dirs::config_dir().unwrap()
    };
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    config_dir.push(PKG_NAME);
    config_dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file() {
        let mut file = if cfg!(target_os = "macos") {
            let home = env::var("HOME").unwrap();
            let mut home_dir = PathBuf::from(home);
            home_dir.push(".config");
            home_dir
        } else {
            dirs::config_dir().unwrap()
        };
        const PKG_NAME: &str = env!("CARGO_PKG_NAME");
        const FILE_NAME: &str = concat!(env!("CARGO_PKG_NAME"), ".json");
        file.push(PKG_NAME);
        file.push(FILE_NAME);

        assert_eq!(config_file(), file);
    }

    #[test]
    fn test_config_path() {
        let mut config_dir = if cfg!(target_os = "macos") {
            let home = env::var("HOME").unwrap();
            let mut home_dir = PathBuf::from(home);
            home_dir.push(".config");
            home_dir
        } else {
            dirs::config_dir().unwrap()
        };
        const PKG_NAME: &str = env!("CARGO_PKG_NAME");
        config_dir.push(PKG_NAME);
        let path = config_path();
        println!("default path: {}", path.display());
        assert_eq!(config_dir, config_dir);
    }
}
