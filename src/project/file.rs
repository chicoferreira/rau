use std::fmt::Display;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::{error::AppResult, utils::pollable_future::PollableFuture};

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

    #[derive(Clone)]
    pub struct FileSystem {
        root: AbsolutePathBuf,
    }

    impl FileSystem {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            Ok(Self { root })
        }

        pub fn create_file_watcher(&self) -> AppResult<FileWatcher> {
            FileWatcher::new(self.root.clone())
        }

        pub fn resolve(&self, path: &ProjectFilePath) -> AppResult<PathBuf> {
            let path_buf = self.root.as_ref().join(&path.path); // TODO: error if path is invalid

            if !path_buf.exists() {
                return Err(AppError::FileNotFound(path.clone()));
            }

            Ok(path_buf)
        }

        pub fn read(&self, file_path: &ProjectFilePath) -> PollableFuture<AppResult<Vec<u8>>> {
            let path = self.resolve(file_path);
            PollableFuture::new(async move {
                let path = path?;
                std::fs::read(&path).map_err(Into::into)
            })
        }

        pub fn read_to_string(
            &self,
            file_path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<String>> {
            let path = self.resolve(file_path);
            PollableFuture::new(async move {
                let path = path?;
                std::fs::read_to_string(&path).map_err(Into::into)
            })
        }

        pub fn list_files(
            &self,
            _path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<Vec<ProjectFilePath>>> {
            PollableFuture::new(async move { todo!() })
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use crate::fs::{absolute::AbsolutePathBuf, file_watcher::FileWatcher};

    use super::*;

    #[derive(Clone)]
    pub struct FileSystem {
        root: AbsolutePathBuf,
    }

    impl FileSystem {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            Ok(Self { root })
        }

        pub fn create_file_watcher(&self) -> AppResult<FileWatcher> {
            FileWatcher::new(self.root.clone())
        }

        pub fn read(&self, file_path: &ProjectFilePath) -> PollableFuture<AppResult<Vec<u8>>> {
            let _ = file_path;
            PollableFuture::new(async move { todo!() })
        }

        pub fn read_to_string(
            &self,
            _file_path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<String>> {
            PollableFuture::new(async move { todo!() })
        }

        pub fn list_files(
            &self,
            _path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<Vec<ProjectFilePath>>> {
            PollableFuture::new(async move { todo!() })
        }
    }
}
