use crate::{Result, commands::load_config_manager};

pub fn run() -> Result<()> {
    let manager = load_config_manager()?;
    let path = manager.config_path();
    if !path.exists() {
        println!("✨ Configuration file not found. Creating a default one...");
        manager.load()?;
        println!("✅ Created: {}", path.display());
    } else {
        println!("📂 Current configuration:");
        println!("   {}", path.display());
    }

    Ok(())
}
