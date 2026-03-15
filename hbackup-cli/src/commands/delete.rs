use crate::Result;
use anyhow::bail;
use clap::Args;
use std::io::{self, Write};

use crate::commands::{ProcessCommand, load_config_manager};

#[derive(Args, Debug)]
pub struct DeleteArgs {
    /// Delete multiple jobs by ids. Cannot be used with --all.
    #[arg(value_delimiter = ',', conflicts_with = "all")]
    pub id: Option<Vec<u32>>,
    /// Delete all jobs. Cannot be used with --id.
    #[arg(short, long, conflicts_with = "id")]
    pub all: bool,
    /// Skip interactive confirmation when deleting all jobs
    #[arg(short = 'y')]
    pub yes: bool,
}

impl ProcessCommand for DeleteArgs {
    async fn run(self) -> Result<()> {
        let manager = load_config_manager()?;
        let mut config = manager.load()?;

        if config.jobs.is_empty() {
            println!("No jobs to delete");
            return Ok(());
        }

        if self.all {
            if !self.yes {
                confirm_delete_all()?;
            }
            config.reset_jobs();
            manager.save(&config)?;
            println!("All jobs deleted successfully.");
            return Ok(());
        }

        if let Some(ids) = self.id {
            config.delete(ids)?;
            manager.save(&config)?;
            return Ok(());
        }

        bail!("Either --all or --id must be specified.");
    }
}

fn confirm_delete_all() -> Result<()> {
    loop {
        print!("Are you sure you want to delete all jobs? (y/n): ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        match input.trim().to_lowercase().as_str() {
            "y" => return Ok(()),
            "n" => return Ok(()),
            _ => println!("Invalid input. Please enter 'y' or 'n'."),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{CONFIG_NAME, PKG_NAME};
    use hbackup_core::model::config::ConfigManager;
    use hbackup_core::model::job::{Job, Strategy};
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    async fn with_temp_config<F, Fut>(temp: &tempfile::TempDir, f: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let original_home = std::env::var_os("HOME");
        let original_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let original_userprofile = std::env::var_os("USERPROFILE");
        let original_appdata = std::env::var_os("APPDATA");
        let original_localappdata = std::env::var_os("LOCALAPPDATA");

        let fake_home = temp.path().to_path_buf();

        unsafe {
            if cfg!(windows) {
                std::env::set_var("USERPROFILE", &fake_home);
                std::env::set_var("APPDATA", fake_home.join("AppData").join("Roaming"));
                std::env::set_var("LOCALAPPDATA", fake_home.join("AppData").join("Local"));
            } else {
                std::env::set_var("HOME", &fake_home);
                std::env::set_var("XDG_CONFIG_HOME", fake_home.join(".config"));
            }
        }

        let result = f().await;

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
        if let Some(value) = original_userprofile {
            unsafe {
                std::env::set_var("USERPROFILE", value);
            }
        } else {
            unsafe {
                std::env::remove_var("USERPROFILE");
            }
        }

        if let Some(value) = original_appdata {
            unsafe {
                std::env::set_var("APPDATA", value);
            }
        } else {
            unsafe {
                std::env::remove_var("APPDATA");
            }
        }

        if let Some(value) = original_localappdata {
            unsafe {
                std::env::set_var("LOCALAPPDATA", value);
            }
        } else {
            unsafe {
                std::env::remove_var("LOCALAPPDATA");
            }
        }

        result
    }

    #[tokio::test]
    #[serial]
    async fn delete_by_id_works() -> Result<()> {
        let temp = tempdir()?;
        let _ = with_temp_config(&temp, || async {
            let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME).unwrap();
            let mut config = manager.load().unwrap();

            let source = temp.path().join("source");
            let target = temp.path().join("target");
            fs::create_dir_all(&source).unwrap();
            fs::create_dir_all(&target).unwrap();

            let job = Job {
                id: 0,
                source: source.clone(),
                target: target.clone(),
                strategy: Strategy::Copy,
            };
            let saved = config.add_job(job).unwrap();
            manager.save(&config).unwrap();

            let cmd = DeleteArgs {
                id: Some(vec![saved.id]),
                all: false,
                yes: false,
            };
            cmd.run().await?;

            let config_path = if cfg!(windows) {
                temp.path()
                    .join("AppData")
                    .join("Roaming")
                    .join(PKG_NAME)
                    .join(CONFIG_NAME)
            } else {
                temp.path().join(".config").join(PKG_NAME).join(CONFIG_NAME)
            };
            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("version = \"1.1\""));
            assert!(!content.contains("[[jobs]]"));

            Ok(())
        })
        .await;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn delete_all_with_yes_works() -> Result<()> {
        let temp = tempdir()?;
        let _ = with_temp_config(&temp, || async {
            let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME).unwrap();
            let mut config = manager.load().unwrap();

            let source = temp.path().join("source");
            let target = temp.path().join("target");
            fs::create_dir_all(&source).unwrap();
            fs::create_dir_all(&target).unwrap();

            let job = Job {
                id: 0,
                source: source.clone(),
                target: target.clone(),
                strategy: Strategy::Copy,
            };
            config.add_job(job).unwrap();
            manager.save(&config).unwrap();

            let cmd = DeleteArgs {
                id: None,
                all: true,
                yes: true,
            };
            cmd.run().await?;

            let config_path = if cfg!(windows) {
                temp.path()
                    .join("AppData")
                    .join("Roaming")
                    .join(PKG_NAME)
                    .join(CONFIG_NAME)
            } else {
                temp.path().join(".config").join(PKG_NAME).join(CONFIG_NAME)
            };
            let content = fs::read_to_string(&config_path).unwrap();
            assert!(content.contains("version = \"1.1\""));
            assert!(!content.contains("[[jobs]]"));

            Ok(())
        })
        .await;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn delete_fails_without_args() -> Result<()> {
        let temp = tempdir()?;
        let _ = with_temp_config(&temp, || async {
            let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME).unwrap();
            let mut config = manager.load().unwrap();
            let source = temp.path().join("source");
            let target = temp.path().join("target");
            fs::create_dir_all(&source).unwrap();
            fs::create_dir_all(&target).unwrap();
            let job = Job {
                id: 0,
                source: source.clone(),
                target: target.clone(),
                strategy: Strategy::Copy,
            };
            config.add_job(job).unwrap();
            manager.save(&config).unwrap();

            let cmd = DeleteArgs {
                id: None,
                all: false,
                yes: false,
            };
            let err = cmd.run().await.unwrap_err();
            assert!(format!("{err}").contains("Either --all or --id must be specified."));
            Ok(())
        })
        .await;
        Ok(())
    }
}
