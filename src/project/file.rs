use std::{fmt::Display, path::PathBuf};

use crate::{
    error::{AppError, AppResult},
    fs::{absolute::AbsolutePathBuf, file_watcher::FileWatcher},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFilePath {
    path: String,
}

impl ProjectFilePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

impl Display for ProjectFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

pub trait FileSystem {
    fn read(&self, path: &ProjectFilePath) -> AppResult<Vec<u8>>;
    fn read_to_string(&self, path: &ProjectFilePath) -> AppResult<String>;
    fn list_files(&self, path: &ProjectFilePath) -> AppResult<Vec<ProjectFilePath>>;
    fn file_watcher(&mut self) -> &mut FileWatcher;
}

pub struct NativeFileSystem {
    pub root: AbsolutePathBuf,
    file_watcher: FileWatcher,
}

impl NativeFileSystem {
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

    fn list_files(&self, _path: &ProjectFilePath) -> AppResult<Vec<ProjectFilePath>> {
        todo!()
    }

    fn file_watcher(&mut self) -> &mut FileWatcher {
        &mut self.file_watcher
    }
}
