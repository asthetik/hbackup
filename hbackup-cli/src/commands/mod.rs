use crate::Result;
use crate::constants::{CONFIG_NAME, CONFIG_VERSION, PKG_NAME};
use hbackup_core::model::config::ConfigManager;
pub mod add;
pub mod config;
pub mod delete;
pub mod list;

pub trait ProcessCommand {
    async fn run(self) -> Result<()>;
}

pub fn load_config_manager() -> Result<ConfigManager> {
    let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME, CONFIG_VERSION.into())?;
    Ok(manager)
}
