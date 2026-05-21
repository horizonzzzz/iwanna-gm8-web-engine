use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use crate::{RuntimeFileHost, RuntimeHostError};

#[derive(Debug)]
pub struct MemoryFileHost {
    root: PathBuf,
    files: HashMap<PathBuf, Vec<u8>>,
}

impl MemoryFileHost {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: HashMap::new(),
        }
    }

    fn resolve_read_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    fn resolve_relative_path(&self, relative_path: &Path) -> Result<PathBuf, RuntimeHostError> {
        if relative_path.is_absolute() {
            return Err(RuntimeHostError::invalid_input(
                "absolute temp paths are not allowed",
            ));
        }

        for component in relative_path.components() {
            match component {
                Component::CurDir | Component::Normal(_) => {}
                Component::Prefix(_) | Component::RootDir | Component::ParentDir => {
                    return Err(RuntimeHostError::invalid_input(
                        "temp paths may not escape the configured host root",
                    ));
                }
            }
        }

        Ok(self.root.join(relative_path))
    }
}

impl RuntimeFileHost for MemoryFileHost {
    fn read(&self, path: &Path) -> Result<Vec<u8>, RuntimeHostError> {
        let resolved = self.resolve_read_path(path);
        self.files.get(&resolved).cloned().ok_or_else(|| {
            RuntimeHostError::not_found(format!("missing host file {}", resolved.display()))
        })
    }

    fn write_temp(
        &mut self,
        relative_path: &Path,
        bytes: &[u8],
    ) -> Result<PathBuf, RuntimeHostError> {
        let resolved = self.resolve_relative_path(relative_path)?;
        self.files.insert(resolved.clone(), bytes.to_vec());
        Ok(resolved)
    }

    fn remove_temp(&mut self, relative_path: &Path) -> Result<(), RuntimeHostError> {
        let resolved = self.resolve_relative_path(relative_path)?;
        self.files.remove(&resolved).map(|_| ()).ok_or_else(|| {
            RuntimeHostError::not_found(format!("missing host file {}", resolved.display()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeHostErrorKind;

    #[test]
    fn memory_file_host_rejects_parent_paths() {
        let mut files = MemoryFileHost::new("sandbox");
        let error = files
            .write_temp(Path::new("../escape.bin"), b"data")
            .unwrap_err();

        assert_eq!(error.kind(), RuntimeHostErrorKind::InvalidInput);
    }

    #[test]
    fn memory_file_host_reads_writes_and_removes_temp_paths() {
        let mut files = MemoryFileHost::new("sandbox");
        let written = files
            .write_temp(Path::new("included/game.dat"), b"payload")
            .unwrap();

        assert_eq!(
            written,
            PathBuf::from("sandbox").join("included").join("game.dat")
        );
        assert_eq!(
            files.read(Path::new("included/game.dat")).unwrap(),
            b"payload"
        );

        files.remove_temp(Path::new("included/game.dat")).unwrap();
        let error = files.read(Path::new("included/game.dat")).unwrap_err();
        assert_eq!(error.kind(), RuntimeHostErrorKind::NotFound);
    }
}
