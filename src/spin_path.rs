use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpinPathError {
    #[error("SPIN_PATH is empty")]
    Empty,
    #[error("directory does not exist: {0}")]
    DirNotFound(PathBuf),
    #[error("module not found: {0}")]
    ModuleNotFound(String),
    #[error("user-defined modules cannot use the 'spin-' prefix: {0}")]
    ReservedPrefix(String),
}

#[derive(Debug)]
pub struct SpinPath {
    dirs: Vec<PathBuf>,
}

impl FromStr for SpinPath {
    type Err = SpinPathError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let dirs: Vec<PathBuf> = s
            .split(':')
            .filter(|segment| !segment.is_empty())
            .map(PathBuf::from)
            .collect();

        if dirs.is_empty() {
            return Err(SpinPathError::Empty);
        }

        for dir in &dirs {
            if !dir.is_dir() {
                return Err(SpinPathError::DirNotFound(dir.clone()));
            }
        }

        Ok(Self { dirs })
    }
}

impl SpinPath {
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    pub fn resolve(&self, module_name: &str) -> Result<PathBuf, SpinPathError> {
        if module_name.starts_with("spin-") && !module_name.starts_with("spin-core") {
            return Err(SpinPathError::ReservedPrefix(module_name.to_string()));
        }

        let filename = format!("{module_name}.spin");
        for dir in &self.dirs {
            let candidate = dir.join(&filename);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }

        Err(SpinPathError::ModuleNotFound(module_name.to_string()))
    }
}
