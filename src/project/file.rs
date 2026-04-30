use std::fmt::Display;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::error::AppResult;

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileSystem;

#[cfg(target_arch = "wasm32")]
pub use wasm::FileSystem;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFilePath {
    path: String,
}

impl ProjectFilePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl Display for ProjectFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use crate::error::AppError;
    use crate::fs::{absolute::AbsolutePathBuf, file_watcher::FileWatcher};

    use super::*;

    pub struct FileSystem {
        pub root: AbsolutePathBuf,
        file_watcher: FileWatcher,
    }

    impl FileSystem {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            let file_watcher = FileWatcher::new(root.clone())?;
            Ok(Self { root, file_watcher })
        }

        pub fn resolve(&self, path: &ProjectFilePath) -> AppResult<PathBuf> {
            let path_buf = self.root.as_ref().join(&path.path); // TODO: error if path is invalid

            if !path_buf.exists() {
                return Err(AppError::FileNotFound(path.clone()));
            }

            Ok(path_buf)
        }

        pub fn read(&self, file_path: &ProjectFilePath) -> AppResult<Vec<u8>> {
            let path = self.resolve(file_path)?;
            std::fs::read(&path).map_err(Into::into)
        }

        pub fn read_to_string(&self, file_path: &ProjectFilePath) -> AppResult<String> {
            let path = self.resolve(file_path)?;
            std::fs::read_to_string(&path).map_err(Into::into)
        }

        pub fn list_files(&self, _path: &ProjectFilePath) -> AppResult<Vec<ProjectFilePath>> {
            todo!()
        }

        pub fn file_watcher(&mut self) -> &mut FileWatcher {
            &mut self.file_watcher
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::fs::file_watcher::FileWatcher;

    use super::*;

    pub struct FileSystem {
        file_watcher: FileWatcher,
    }

    impl FileSystem {
        pub fn new() -> AppResult<Self> {
            todo!()
        }

        pub fn read(&self, _file_path: &ProjectFilePath) -> AppResult<Vec<u8>> {
            todo!()
        }

        pub fn read_to_string(&self, _file_path: &ProjectFilePath) -> AppResult<String> {
            todo!()
        }

        pub fn list_files(&self, _path: &ProjectFilePath) -> AppResult<Vec<ProjectFilePath>> {
            todo!()
        }

        pub fn file_watcher(&mut self) -> &mut FileWatcher {
            &mut self.file_watcher
        }
    }
}
