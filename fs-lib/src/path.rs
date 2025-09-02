use std::path::PathBuf;

use anyhow::Result;

pub trait PathExtension {
    fn convert_relative_path(&self, source: &PathBuf, destination: &PathBuf) -> Result<PathBuf>;

    fn get_relative_path(&self, target: &PathBuf) -> Result<PathBuf>;
}

impl PathExtension for PathBuf {
    fn convert_relative_path(&self, source: &PathBuf, destination: &PathBuf) -> Result<PathBuf> {
        let path = source.get_relative_path(self)?;

        Ok(destination.join(path))
    }

    fn get_relative_path(&self, target: &PathBuf) -> Result<PathBuf> {
        let path = target.strip_prefix(self)?;

        Ok(PathBuf::from(path))
    }
}
