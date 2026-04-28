use std::path::PathBuf;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFilePath {
    path: String,
}

impl ProjectFilePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl ToString for ProjectFilePath {
    fn to_string(&self) -> String {
        self.path.clone()
    }
}

pub trait FileSystem {
    fn read(&self, path: &ProjectFilePath) -> AppResult<Vec<u8>>;
    fn read_to_string(&self, path: &ProjectFilePath) -> AppResult<String>;
}

pub struct NativeFileSystem {
    pub root: PathBuf,
}

impl NativeFileSystem {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn resolve(&self, path: &ProjectFilePath) -> AppResult<PathBuf> {
        let path_buf = self.root.join(&path.path); // TODO: error if path is invalid

        if !path_buf.exists() {
            return Err(AppError::FileNotFound(path.clone()));
        }

        Ok(path_buf)
    }
}

impl FileSystem for NativeFileSystem {
    fn read(&self, file_path: &ProjectFilePath) -> AppResult<Vec<u8>> {
        let path = self.resolve(file_path)?;
        std::fs::read(&path).map_err(Into::into)
    }

    fn read_to_string(&self, file_path: &ProjectFilePath) -> AppResult<String> {
        let path = self.resolve(file_path)?;
        std::fs::read_to_string(&path).map_err(Into::into)
    }
}
