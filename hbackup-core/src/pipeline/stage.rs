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
            let pattern = if rule.starts_with('!') {
                rule.to_string()
            } else {
                format!("!{}", rule)
            };
            overrides.add(&pattern)?;
        }

        let overrides = overrides.build()?;

        let walker = WalkBuilder::new(&self.source)
            .overrides(overrides)
            .standard_filters(false)
            .build();

        let items = walker
            .filter_map(|entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => return Some(Err(HbackupError::from(e))),
                };

                if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    return None;
                }

                let absolute = entry.into_path();
                match absolute.strip_prefix(&self.source) {
                    Ok(rel) => {
                        let relative = rel.to_path_buf();
                        Some(Ok(ScannedFile::new(absolute, relative)))
                    }
                    Err(e) => Some(Err(HbackupError::RuntimeError(format!(
                        "Path alignment error: {}",
                        e
                    )))),
                }
            })
            .collect::<Result<Vec<ScannedFile>>>()?;

        Ok(items)
    }
}
