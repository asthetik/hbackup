use std::{
    collections::HashSet,
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
        let default_file = default_file();
        if path::check_path(&default_file).is_err() {
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
        let default_file = default_file();
        if fs::metadata(&default_file).is_err() {
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
        let path = default_file();
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
        let path = default_path();
        if fs::metadata(&path).is_err() {
            fs::create_dir_all(&path)?;
        }
        let file = default_file();
        let file = fs::File::create(&file)?;
        let writer = io::BufWriter::new(file);
        let tasks = vec![self];
        serde_json::to_writer_pretty(writer, &tasks)?;
        Ok(())
    }

    fn write(tasks: &[Task]) -> Result<(), Box<dyn Error>> {
        let path = default_file();
        let file = fs::File::create(path)?;
        let writer = io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, tasks)?;
        Ok(())
    }
}

fn default_file() -> PathBuf {
    let mut path = default_path();
    path.push("tasks.json");
    path
}

fn default_path() -> PathBuf {
    path::expand_path("~/.config/hbackup")
}
