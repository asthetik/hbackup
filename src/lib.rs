//! hbackup: A high-performance, cross-platform CLI backup tool.
//!
//! This crate provides the core logic, command definitions, and path utilities for the hbackup CLI application.

use std::error::Error;

pub mod application;
pub mod commands;
pub mod path;
pub mod sysexits;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
