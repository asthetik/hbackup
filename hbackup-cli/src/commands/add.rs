use crate::Result;
use crate::commands::ProcessCommand;
use crate::constants::{CONFIG_NAME, PKG_NAME};
use clap::Args;
use hbackup_core::model::config::ConfigManager;
use hbackup_core::model::job::{ArchiveFormat, Job, Level, Strategy};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Source directory or file to back up
    pub source: PathBuf,

    /// Target directory where the backup will be stored
    pub target: PathBuf,

    /// Backup mode: mirror, copy, or archive
    #[arg(short, long, value_enum, default_value_t = CliStrategy::Copy)]
    pub mode: CliStrategy,

    /// Compression format (required only for archive mode)
    #[arg(
        short,
        long,
        value_enum,
        requires_if("mode", "archive"),
        default_value_t = ArchiveFormat::Tar
    )]
    pub format: ArchiveFormat,

    /// Compression level (required only for archive mode)
    #[arg(
        short,
        long,
        value_enum,
        requires_if("mode", "archive"),
        default_value_t = Level::Default
    )]
    pub level: Level,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum CliStrategy {
    Mirror,
    Copy,
    Archive,
}

impl ProcessCommand for AddArgs {
    async fn run(self) -> Result<()> {
        let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME)?;

        let strategy = match self.mode {
            CliStrategy::Mirror => Strategy::Mirror,
            CliStrategy::Copy => Strategy::Copy,
            CliStrategy::Archive => Strategy::Archive {
                format: self.format,
                level: self.level,
            },
        };
        let new_job = Job {
            id: 0,
            source: self.source,
            target: self.target,
            strategy,
        };
        new_job.validate()?;

        let mut config = manager.load()?;
        let saved_job = config.add_job(new_job)?;
        manager.save(&config)?;
        println!("✅ Job added successfully!");
        println!("ID: {}", saved_job.id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn clean_windows_path(original: &Path) -> PathBuf {
        if cfg!(windows) {
            let s = original.display().to_string();
            if let Some(stripped) = s.strip_prefix(r"\\?\\") {
                PathBuf::from(stripped)
            } else {
                original.to_path_buf()
            }
        } else {
            original.to_path_buf()
        }
    }

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

        let fake_home = clean_windows_path(temp.path());

        unsafe {
            if cfg!(windows) {
                std::env::set_var("HOME", &fake_home);
                std::env::set_var("USERPROFILE", &fake_home);
                std::env::set_var("APPDATA", fake_home.join("AppData").join("Roaming"));
                std::env::set_var("LOCALAPPDATA", fake_home.join("AppData").join("Local"));
                std::fs::create_dir_all(fake_home.join("AppData").join("Roaming")).unwrap();
                std::fs::create_dir_all(fake_home.join("AppData").join("Local")).unwrap();
            } else {
                std::env::set_var("HOME", &fake_home);
                std::env::set_var("XDG_CONFIG_HOME", fake_home.join(".config"));
                std::fs::create_dir_all(fake_home.join(".config")).unwrap();
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
    async fn test_add_command_isolated() -> Result<()> {
        let tmp = tempdir()?;

        let _ = with_temp_config(&tmp, || async {
            let fake_home_path = clean_windows_path(tmp.path());

            // 3. Prepare dummy source and target directories for the test
            let source_dir = fake_home_path.join("data_to_back_up");
            let target_dir = fake_home_path.join("backup_vault");
            println!("--- Test Path Debug ---");
            println!("Source Directory: {:?}", source_dir);
            println!("Target Directory: {:?}", target_dir);
            println!("-----------------------");
            fs::create_dir(&source_dir)?;
            fs::create_dir(&target_dir)?;

            // 4. Construct AddArgs instance manually
            let args = AddArgs {
                source: source_dir,
                target: target_dir,
                mode: CliStrategy::Mirror,
                format: ArchiveFormat::Tar,
                level: Level::Default,
            };

            // 5. Execute the command logic
            args.run().await?;

            let config_path = if cfg!(windows) {
                std::env::var_os("APPDATA")
                    .map(PathBuf::from)
                    .expect("APPDATA should be set in with_temp_config")
                    .join(PKG_NAME)
                    .join("config.toml")
            } else {
                let base = std::env::var_os("XDG_CONFIG_HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| std::env::var_os("HOME").unwrap().into());
                base.join(PKG_NAME).join("config.toml")
            };
            println!("config_path {:?}", config_path);
            // Assert that the config file exists
            assert!(
                config_path.exists(),
                "Config file should exist at {:?}",
                config_path
            );

            // Assert that the content contains the "jobs" key
            let content = fs::read_to_string(config_path)?;
            println!("content: {content}");
            assert!(
                content.contains("jobs"),
                "Config file should contain 'jobs'"
            );

            Ok(())
        })
        .await;

        Ok(())
    }
}
