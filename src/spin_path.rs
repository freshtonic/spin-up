use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpinPathError {
    #[error("SPIN_PATH is empty")]
    Empty,
    #[error("directory does not exist: {0}")]
    DirNotFound(PathBuf),
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
}
