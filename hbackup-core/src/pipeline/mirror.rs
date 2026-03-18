use crate::error::Result;
use crate::model::job::Strategy;
use crate::pipeline::stage::Scanner;
use std::fs;
use std::path::PathBuf;

pub struct SyncExecutor {
    source: PathBuf,
    target: PathBuf,
    ignore_rules: Vec<String>,
}

impl SyncExecutor {
    pub fn new(source: PathBuf, target: PathBuf, ignore_rules: Vec<String>) -> Self {
        Self {
            source,
            target,
            ignore_rules,
        }
    }

    pub fn run(&self, strategy: Strategy) -> Result<()> {
        let scanner = Scanner::new(self.source.clone(), self.ignore_rules.clone());
        let files = scanner.scan()?;

        for file in files {
            let dest = self.target.join(&file.relative);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&file.absolute, &dest)?;
        }

        if matches!(strategy, Strategy::Mirror) {
            self.cleanup_target_extras()?;
        }

        Ok(())
    }

    fn cleanup_target_extras(&self) -> Result<()> {
        todo!()
    }
}
