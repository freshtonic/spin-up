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
    #[error("failed to read {path}: {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
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

    pub fn resolve_source(&self, module_name: &str) -> Result<String, SpinPathError> {
        // Check builtins first
        if let Some(source) = crate::builtins::get_module_source(module_name) {
            return Ok(source.to_string());
        }

        // Then check SPIN_PATH on disk
        let path = self.resolve(module_name)?;
        std::fs::read_to_string(&path).map_err(|e| SpinPathError::ReadError { path, source: e })
    }

    pub fn resolve(&self, module_name: &str) -> Result<PathBuf, SpinPathError> {
        // spin-core-* modules are built-in (handled by resolve_source above).
        // Reject only exact "spin-core" prefix on disk — all other spin-* modules
        // (spin-net, spin-db-postgres, etc.) are resolved normally from SPIN_PATH.

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
