use crate::Result;
use crate::constants::{CONFIG_NAME, PKG_NAME};
use hbackup_core::model::config::ConfigManager;
pub mod add;
pub mod delete;

pub trait ProcessCommand {
    async fn run(self) -> Result<()>;
}

pub fn load_config_manager() -> Result<ConfigManager> {
    let manager = ConfigManager::new(PKG_NAME, CONFIG_NAME)?;
    Ok(manager)
}
