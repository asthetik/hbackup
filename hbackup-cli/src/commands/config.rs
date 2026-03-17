use crate::{Result, commands::load_config_manager};

pub fn run() -> Result<()> {
    let config_path = load_config_manager()?;
    println!("{}", config_path.config_path().display());
    Ok(())
}
