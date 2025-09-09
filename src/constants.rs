/// Package name.
pub(crate) const PKG_NAME: &str = env!("CARGO_PKG_NAME");
/// Default configuration file name.
pub(crate) const CONFIG_NAME: &str = "config.toml";
/// Backup configuration file name.
pub(crate) const CONFIG_BACKUP_NAME: &str = "config_backup.toml";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_name() {
        assert_eq!(PKG_NAME, "hbackup");
    }

    #[test]
    fn test_config_name() {
        assert_eq!(CONFIG_NAME, "config.toml");
        assert!(CONFIG_NAME.ends_with(".toml"));
    }

    #[test]
    fn test_config_backup_name() {
        assert_eq!(CONFIG_BACKUP_NAME, "config_backup.toml");
        assert!(CONFIG_BACKUP_NAME.ends_with(".toml"));
        assert!(CONFIG_BACKUP_NAME.contains("backup"));
    }

    #[test]
    fn test_config_names_are_different() {
        assert_ne!(CONFIG_NAME, CONFIG_BACKUP_NAME);
    }

    #[test]
    fn test_constants_are_valid_filenames() {
        // Test that constants don't contain invalid filename characters
        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

        for &invalid_char in &invalid_chars {
            assert!(!CONFIG_NAME.contains(invalid_char));
            assert!(!CONFIG_BACKUP_NAME.contains(invalid_char));
        }
    }
}
