use std::error::Error;

/// Unified result type for all fallible operations in hbackup.
pub(crate) type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Default configuration file name.
pub(crate) const CONFIG_NAME: &str = "config.toml";
/// Backup configuration file name.
pub(crate) const CONFIG_BACKUP_NAME: &str = "config_backup.toml";
