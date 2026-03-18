use crate::error::{HbackupError, Result};
use ignore::{WalkBuilder, overrides::OverrideBuilder};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Scanner {
    source: PathBuf,
    ignore_rules: Vec<String>,
}

#[derive(Debug)]
pub struct ScannedFile {
    pub absolute: PathBuf,
    pub relative: PathBuf,
}

impl ScannedFile {
    pub fn new(absolute: PathBuf, relative: PathBuf) -> Self {
        Self { absolute, relative }
    }
}

impl Scanner {
    pub fn new(source: PathBuf, ignore_rules: Vec<String>) -> Self {
        Self {
            source,
            ignore_rules,
        }
    }
    pub fn scan(&self) -> Result<Vec<ScannedFile>> {
        let mut overrides = OverrideBuilder::new(&self.source);
        for rule in &self.ignore_rules {
            let pattern = if rule.starts_with("!") {
                rule.to_string()
            } else {
                format!("!{}", rule)
            };
            overrides.add(&pattern)?;
        }

        let overrides = overrides.build()?;
        let files = WalkBuilder::new(&self.source)
            .overrides(overrides)
            .standard_filters(false)
            .build();

        let mut items = vec![];
        for entry in files {
            let file = entry?;

            if !file.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let absolute = file.into_path();
            let relative = absolute
                .strip_prefix(&self.source)
                .map_err(|e| HbackupError::RuntimeError(format!("Path alignment error: {}", e)))?
                .to_path_buf();

            items.push(ScannedFile::new(absolute, relative));
        }

        Ok(items)
    }
}
