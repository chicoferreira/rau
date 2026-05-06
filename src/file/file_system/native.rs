use std::path::PathBuf;

use crate::{
    error::{AppError, AppResult},
    file::{
        file_system::{FileSystemEntries, FileSystemTrait, FutureResult},
        file_watcher::FileWatcher,
        identifier::ProjectIdentifier,
    },
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

#[derive(Clone)]
pub struct FileSystem {}

impl FileSystem {
    fn resolve(&self, identifier: &ProjectIdentifier, file_path: &FilePath) -> PathBuf {
        let mut path_buf = identifier.project_path().as_path_buf();
        for segment in file_path.segments() {
            path_buf = path_buf.join(segment);
        }

        path_buf
    }

    fn resolve_exists(
        &self,
        identifier: &ProjectIdentifier,
        file_path: &FilePath,
    ) -> AppResult<PathBuf> {
        let path_buf = self.resolve(identifier, file_path);
        if !path_buf.try_exists()? {
            return Err(AppError::FileNotFound(file_path.clone()));
        }

        Ok(path_buf)
    }
}

impl FileSystemTrait for FileSystem {
    fn new() -> FutureResult<Self> {
        AsyncJob::new(async move { Ok(Self {}) })
    }

    fn create_file_watcher(&self, id: &ProjectIdentifier) -> AppResult<FileWatcher> {
        FileWatcher::new(id.project_path().clone())
    }

    fn read(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<u8>> {
        let path = self.resolve_exists(id, path);
        AsyncJob::new(async move {
            let path = path?;
            std::fs::read(&path).map_err(Into::into)
        })
    }

    fn read_to_string(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<String> {
        let path = self.resolve_exists(id, path);
        AsyncJob::new(async move {
            let path = path?;
            std::fs::read_to_string(&path).map_err(Into::into)
        })
    }

    fn list_entries(&self, id: &ProjectIdentifier) -> FutureResult<FileSystemEntries> {
        let root = id.project_path().as_path_buf();

        AsyncJob::new(async move {
            let mut files = Vec::new();
            let mut directories = Vec::new();

            collect_entries(&root, &root, &mut files, &mut directories)?;
            files.sort_by_key(|file| file.segments().to_vec());
            directories.sort_by_key(|directory| directory.segments().to_vec());

            Ok(FileSystemEntries { files, directories })
        })
    }

    fn create_directory(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()> {
        let resolved_path = self.resolve(id, path);
        let path = path.clone();

        AsyncJob::new(async move {
            if resolved_path.try_exists()? {
                return Err(AppError::PathAlreadyExists(path));
            }

            std::fs::create_dir_all(resolved_path).map_err(Into::into)
        })
    }

    fn save(&self, id: &ProjectIdentifier, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let path = self.resolve(id, path);

        AsyncJob::new(async move {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(path, bytes).map_err(Into::into)
        })
    }

    fn create_empty_file(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()> {
        let resolved_path = self.resolve(id, path);
        let path = path.clone();

        AsyncJob::new(async move {
            if resolved_path.try_exists()? {
                return Err(AppError::PathAlreadyExists(path.clone()));
            }

            if let Some(parent) = resolved_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(resolved_path)
            {
                Ok(_) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    Err(AppError::PathAlreadyExists(path))
                }
                Err(err) => Err(err.into()),
            }
        })
    }

    fn delete_path(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<FilePath>> {
        let root = id.project_path().as_path_buf();
        let resolved_path = self.resolve_exists(id, path);
        let path = path.clone();

        AsyncJob::new(async move {
            let resolved_path = resolved_path?;
            if resolved_path.is_dir() {
                let mut files = Vec::new();
                let mut directories = vec![path];
                collect_entries(&root, &resolved_path, &mut files, &mut directories)?;

                std::fs::remove_dir_all(resolved_path)?;

                Ok(directories.into_iter().chain(files).collect())
            } else if resolved_path.is_file() {
                std::fs::remove_file(resolved_path)?;

                Ok(vec![path])
            } else {
                Err(AppError::FileNotFound(path))
            }
        })
    }

    fn move_path(
        &self,
        id: &ProjectIdentifier,
        old: &FilePath,
        new: &FilePath,
    ) -> FutureResult<Vec<FilePath>> {
        let root = id.project_path().as_path_buf();
        let old_resolved_path = self.resolve_exists(id, old);
        let new_resolved_path = self.resolve(id, new);
        let old = old.clone();
        let new = new.clone();

        AsyncJob::new(async move {
            if old == new {
                return Ok(Vec::new());
            }

            let old_resolved_path = old_resolved_path?;
            if new_resolved_path.try_exists()? {
                return Err(AppError::PathAlreadyExists(new));
            }

            let mut changed_paths = Vec::new();
            if old_resolved_path.is_dir() {
                if new.starts_with(&old) {
                    return Err(AppError::PathAlreadyExists(new));
                }

                let mut files = Vec::new();
                let mut directories = vec![old.clone()];
                collect_entries(&root, &old_resolved_path, &mut files, &mut directories)?;

                changed_paths.extend(directories.iter().cloned());
                changed_paths.extend(files.iter().cloned());
                changed_paths.extend(
                    directories
                        .iter()
                        .chain(files.iter())
                        .filter_map(|path| path.replace_prefix(&old, &new)),
                );
            } else {
                changed_paths.push(old);
                changed_paths.push(new.clone());
            }

            if let Some(parent) = new_resolved_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::rename(old_resolved_path, new_resolved_path)?;

            Ok(changed_paths)
        })
    }
}

fn collect_entries(
    root: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<FilePath>,
    directories: &mut Vec<FilePath>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");

            directories.push(FilePath::from_relative_path(relative_path));
            collect_entries(root, &path, files, directories)?;
        } else if file_type.is_file() {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            let relative_path = relative_path.to_string_lossy().replace('\\', "/");

            files.push(FilePath::from_relative_path(relative_path));
        }
    }

    Ok(())
}
