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
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_add_command_isolated() -> Result<()> {
        // 1. Create a fully isolated temporary directory to act as a "fake home"
        let tmp = tempdir()?;
        let fake_home_path = fs::canonicalize(tmp.path())?;

        // 2. Redirect the HOME/USERPROFILE/XDG_CONFIG_HOME environment variables
        // This trick ensures ConfigManager looks for config files inside the temp dir
        // instead of your actual system home directory.
        let original_home = std::env::var_os("HOME");
        let original_xdg = std::env::var_os("XDG_CONFIG_HOME");
        let original_userprofile = std::env::var_os("USERPROFILE");

        unsafe {
            if cfg!(windows) {
                std::env::set_var("USERPROFILE", &fake_home_path);
            } else {
                std::env::set_var("HOME", &fake_home_path);
                std::env::set_var("XDG_CONFIG_HOME", fake_home_path.join(".config"));
            }
        }

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
            fake_home_path
                .join("AppData")
                .join("Roaming")
                .join(PKG_NAME)
                .join("config.toml")
        } else {
            fake_home_path
                .join(".config")
                .join(PKG_NAME)
                .join("config.toml")
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

        // restore environment
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

        Ok(())
    }
}
