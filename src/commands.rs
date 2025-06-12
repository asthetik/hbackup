use std::{error::Error, fs};

use clap::{Parser, Subcommand};

use crate::{
    path,
    task::{self, Task},
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Option<Commands>,
}

/// hbackup commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// create a backup task
    Create {
        /// source file
        #[arg(short, long)]
        source: String,
        /// target file or directory
        #[arg(short, long)]
        target: String,
        /// task id
        #[arg(short, long)]
        id: Option<u32>,
    },
    /// run all backup task
    Run,
    /// list all tasks
    List,
    /// delete tasks
    Delete {
        /// delete task by id
        #[arg(short, long, required = false, conflicts_with = "all")]
        id: Option<u32>,
        /// delete all tasks
        #[arg(long, required = false, conflicts_with = "id")]
        all: bool,
    },
    /// Display the configuration file path
    Config,
}

pub fn create(source: String, target: String, id: Option<u32>) -> Result<(), Box<dyn Error>> {
    let source = path::expand_path(&source);
    let target = path::expand_path(&target);
    path::check_path(&source)?;
    let task = Task::new(id, source, target);
    task.save()?;
    Ok(())
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let tasks = Task::get_all()?;
    if tasks.is_empty() {
        println!("No tasks are backed up!");
        return Ok(());
    }
    for task in tasks {
        let target_file = if task.target.exists() && task.target.is_dir() {
            let file_name = task.source.file_name().ok_or("invalid source file name")?;
            let mut target = task.target.clone();
            target.push(file_name);
            target
        } else {
            task.target.clone()
        };

        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent)?;
        }

        match fs::copy(&task.source, &target_file) {
            Ok(_) => println!(
                "Task id: {} from {} to {} backed up successfully.",
                task.id,
                task.source.display(),
                target_file.display()
            ),
            Err(e) => eprintln!(
                "Failed to backup task id: {} from {} to {}: {}",
                task.id,
                task.source.display(),
                target_file.display(),
                e
            ),
        }
    }
    Ok(())
}

pub fn list() -> Result<(), Box<dyn Error>> {
    let tasks = Task::get_all()?;
    println!("{tasks:#?}");
    Ok(())
}

pub fn delete(id: Option<u32>, all: bool) -> Result<(), Box<dyn Error>> {
    if all {
        Task::delete_all()?;
        println!("All tasks deleted successfully.");
    } else if let Some(id) = id {
        Task::delete_by_id(id)?;
        println!("Task with id {id} deleted successfully.");
    } else {
        return Err("Either --all or --id must be specified.".into());
    }
    Ok(())
}

pub fn config() {
    println!("config file: {}", task::config_file().display());
}
