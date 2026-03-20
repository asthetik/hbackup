/// Package name.
pub(crate) const PKG_NAME: &str = env!("CARGO_PKG_NAME");
/// Default configuration file name.
pub(crate) const CONFIG_NAME: &str = "config.toml";

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
}
