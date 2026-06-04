use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, Mutex},
};

use crate::{
    error::{AppError, AppResult},
    file::file_system::{FileSystemEntries, FutureResult, ProjectFileSystemTrait},
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

#[derive(Clone, Default)]
pub struct EphemeralFileSystem {
    state: Arc<Mutex<EphemeralFileSystemState>>,
}

#[derive(Default)]
struct EphemeralFileSystemState {
    files: BTreeMap<FilePath, Vec<u8>>,
    directories: BTreeSet<FilePath>,
}

impl EphemeralFileSystemState {
    fn contains_entry(&self, path: &FilePath) -> bool {
        path.segments().is_empty()
            || self.files.contains_key(path)
            || self.directories.contains(path)
    }

    fn ensure_directories(&mut self, path: &FilePath) {
        self.directories.extend(path.ancestors_inclusive());
    }

    fn ensure_parent_directories(&mut self, path: &FilePath) {
        if let Some(parent) = path.parent() {
            self.ensure_directories(&parent);
        }
    }
}

impl EphemeralFileSystem {
    fn run<T: Send + 'static>(
        &self,
        function: impl FnOnce(&mut EphemeralFileSystemState) -> AppResult<T> + Send + 'static,
    ) -> FutureResult<T> {
        let state = self.state.clone();
        AsyncJob::new(async move {
            let mut state = state.lock().expect("ephemeral file system state poisoned");
            function(&mut state)
        })
    }
}

impl ProjectFileSystemTrait for EphemeralFileSystem {
    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>> {
        let path = path.clone();

        self.run(move |state| {
            state
                .files
                .get(&path)
                .cloned()
                .ok_or(AppError::FileNotFound(path))
        })
    }

    fn exists(&self, path: &FilePath) -> FutureResult<bool> {
        let path = path.clone();

        self.run(move |state| Ok(state.contains_entry(&path)))
    }

    fn list_entries(&self) -> FutureResult<FileSystemEntries> {
        self.run(|state| {
            Ok(FileSystemEntries {
                files: state.files.keys().cloned().collect(),
                directories: state.directories.iter().cloned().collect(),
            })
        })
    }

    fn create_directory(&self, path: &FilePath) -> FutureResult<()> {
        let path = path.clone();

        self.run(move |state| {
            if state.contains_entry(&path) {
                return Err(AppError::PathAlreadyExists(path));
            }

            state.ensure_directories(&path);
            Ok(())
        })
    }

    fn write(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let path = path.clone();

        self.run(move |state| {
            if state.directories.contains(&path) || path.segments().is_empty() {
                return Err(AppError::PathAlreadyExists(path));
            }

            state.ensure_parent_directories(&path);
            state.files.insert(path, bytes);
            Ok(())
        })
    }

    fn delete_path(&self, path: &FilePath) -> FutureResult<()> {
        let path = path.clone();

        self.run(move |state| {
            if path.segments().is_empty() {
                state.files.clear();
                state.directories.clear();
                return Ok(());
            }

            if state.files.remove(&path).is_some() {
                return Ok(());
            }

            if !state.directories.contains(&path) {
                return Err(AppError::FileNotFound(path));
            }

            state
                .files
                .retain(|file_path, _| !file_path.starts_with(&path));
            state
                .directories
                .retain(|directory_path| !directory_path.starts_with(&path));
            Ok(())
        })
    }

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()> {
        let old = old.clone();
        let new = new.clone();

        self.run(move |state| {
            if old == new {
                return Ok(());
            }

            if new.starts_with(&old) || state.contains_entry(&new) {
                return Err(AppError::PathAlreadyExists(new));
            }

            if let Some(bytes) = state.files.remove(&old) {
                state.ensure_parent_directories(&new);
                state.files.insert(new, bytes);
                return Ok(());
            }

            if !state.directories.contains(&old) {
                return Err(AppError::FileNotFound(old));
            }

            state.ensure_parent_directories(&new);

            let moved_files = state
                .files
                .iter()
                .filter_map(|(path, bytes)| {
                    path.replace_prefix(&old, &new)
                        .map(|moved_path| (path.clone(), moved_path, bytes.clone()))
                })
                .collect::<Vec<_>>();
            let moved_directories = state
                .directories
                .iter()
                .filter_map(|path| {
                    path.replace_prefix(&old, &new)
                        .map(|moved_path| (path.clone(), moved_path))
                })
                .collect::<Vec<_>>();

            for (path, _, _) in &moved_files {
                state.files.remove(path);
            }
            for (path, _) in &moved_directories {
                state.directories.remove(path);
            }
            for (_, moved_path, bytes) in moved_files {
                state.files.insert(moved_path, bytes);
            }
            for (_, moved_path) in moved_directories {
                state.directories.insert(moved_path);
            }

            Ok(())
        })
    }
}
