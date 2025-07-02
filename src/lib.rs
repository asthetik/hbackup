//! hbackup: A high-performance, cross-platform CLI backup tool.
//!
//! This crate provides the core logic, command definitions, file utilities, and path handling
//! for the hbackup CLI application. It exposes modules for configuration management, command-line
//! parsing, file compression, and system exit codes. All error handling is unified via the `Result` type alias.
//!
//! # Modules
//! - [`application`]: Application configuration and job management
//! - [`commands`]: CLI command definitions and handlers
//! - [`file_util`]: File and directory compression utilities
//! - [`path`]: Path expansion and validation helpers
//! - [`sysexits`]: Standardized system exit codes
//!
//! # Constants
//! - `CONFIG_NAME`: Default configuration file name
//! - `CONFIG_BACKUP_NAME`: Backup configuration file name
//!
//! # Example
//! ```no_run
//! use hbackup::application::Application;
//! let app = Application::new();
//! ```

use std::error::Error;

pub mod application;
pub mod commands;
pub mod file_util;
pub mod path;
pub mod sysexits;

/// Unified result type for all fallible operations in hbackup.
pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Default configuration file name.
pub const CONFIG_NAME: &str = "config.toml";
/// Backup configuration file name.
pub const CONFIG_BACKUP_NAME: &str = "config_backup.toml";
