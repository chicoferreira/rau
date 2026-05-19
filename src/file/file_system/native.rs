use std::{collections::HashSet, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    file::{
        absolute::AbsolutePathBuf,
        file_system::{
            AppFileSystemTrait, FileSystemEntries, FutureResult, ProjectFileSystemTrait,
        },
        file_watcher::FileWatcher,
        identifier::ProjectIdentifier,
    },
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

type FileSystemJob = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone)]
pub struct AppFileSystem {
    send_jobs: std::sync::mpsc::Sender<FileSystemJob>,
}

#[derive(Clone)]
pub struct ProjectFileSystem {
    id: ProjectIdentifier,
    send_jobs: std::sync::mpsc::Sender<FileSystemJob>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub recent_projects: Vec<AbsolutePathBuf>,
}

fn config_path() -> AppResult<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or(AppError::ConfigDirectoryUnavailable)?
        .join("rau")
        .join("config.toml"))
}

impl AppConfig {
    fn read_or_default() -> AppResult<Self> {
        let path = config_path()?;
        match std::fs::read_to_string(&path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn save(&self) -> AppResult<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;

        Ok(())
    }

    fn remember_project_path(&mut self, project_path: AbsolutePathBuf) {
        self.recent_projects.retain(|path| path != &project_path);
        self.recent_projects.insert(0, project_path);
    }
}

impl AppFileSystemTrait for AppFileSystem {
    fn open() -> FutureResult<Self> {
        AsyncJob::new(async move {
            let (send_jobs, receive_jobs) = std::sync::mpsc::channel();
            spawn_worker_thread(receive_jobs);

            Ok(Self { send_jobs })
        })
    }

    fn mount_project(
        &self,
        id: ProjectIdentifier,
    ) -> FutureResult<(ProjectFileSystem, FileWatcher)> {
        let send_jobs = self.send_jobs.clone();

        AsyncJob::new(async move {
            std::fs::create_dir_all(id.project_path())?; // File watcher requires the directory to exist

            let file_watcher = FileWatcher::new(id.project_path().clone())?;
            let file_system = ProjectFileSystem { id, send_jobs };

            Ok((file_system, file_watcher))
        })
    }

    fn recent_projects(&self) -> FutureResult<Vec<ProjectIdentifier>> {
        spawn_blocking(self.send_jobs.clone(), move || {
            let config = AppConfig::read_or_default()?;
            let mut seen = HashSet::new();
            let mut projects = Vec::new();

            for path in config.recent_projects {
                match ProjectIdentifier::extract_identifier(path) {
                    Ok(project_identifier) => {
                        let project_path = project_identifier.project_path().as_path_buf();
                        if seen.insert(project_path) {
                            projects.push(project_identifier);
                        }
                    }
                    Err(error) => {
                        log::error!("Skipping invalid recent project: {error}");
                    }
                }
            }

            Ok(projects)
        })
    }

    fn remember_project(&self, id: ProjectIdentifier) -> FutureResult<()> {
        let project_path = id.project_path().clone();

        spawn_blocking(self.send_jobs.clone(), move || {
            let mut config = AppConfig::read_or_default()?;
            config.remember_project_path(project_path);
            config.save()
        })
    }

    fn remove_recent_project(&self, id: ProjectIdentifier) -> FutureResult<()> {
        let project_path = id.project_path().clone();

        spawn_blocking(self.send_jobs.clone(), move || {
            let mut config = AppConfig::read_or_default()?;
            config.recent_projects.retain(|path| path != &project_path);
            config.save()
        })
    }
}

impl ProjectFileSystem {
    fn resolve(&self, file_path: &FilePath) -> PathBuf {
        let mut path_buf = self.id.project_path().as_path_buf();
        for segment in file_path.segments() {
            path_buf = path_buf.join(segment);
        }

        path_buf
    }

    fn resolve_exists(&self, file_path: &FilePath) -> AppResult<PathBuf> {
        let path_buf = self.resolve(file_path);
        if !path_buf.try_exists()? {
            return Err(AppError::FileNotFound(file_path.clone()));
        }

        Ok(path_buf)
    }

    fn run_blocking<T: Send + 'static>(
        &self,
        function: impl FnOnce() -> T + Send + 'static,
    ) -> AsyncJob<T> {
        spawn_blocking(self.send_jobs.clone(), function)
    }
}

impl ProjectFileSystemTrait for ProjectFileSystem {
    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>> {
        let path = self.resolve_exists(path);

        self.run_blocking(|| {
            let path = path?;
            std::fs::read(&path).map_err(Into::into)
        })
    }

    fn read_to_string(&self, path: &FilePath) -> FutureResult<String> {
        let path = self.resolve_exists(path);
        self.run_blocking(|| {
            let path = path?;
            std::fs::read_to_string(&path).map_err(Into::into)
        })
    }

    fn exists(&self, path: &FilePath) -> FutureResult<bool> {
        let path = self.resolve(path);

        self.run_blocking(move || path.try_exists().map_err(Into::into))
    }

    fn list_entries(&self) -> FutureResult<FileSystemEntries> {
        let root = self.id.project_path().as_path_buf();
        self.run_blocking(move || {
            let mut files = Vec::new();
            let mut directories = Vec::new();

            collect_entries(&root, &root, &mut files, &mut directories)?;
            files.sort_by_key(|file| file.segments().to_vec());
            directories.sort_by_key(|directory| directory.segments().to_vec());

            Ok(FileSystemEntries { files, directories })
        })
    }

    fn create_directory(&self, path: &FilePath) -> FutureResult<()> {
        let resolved_path = self.resolve(path);
        let path = path.clone();

        self.run_blocking(|| {
            if resolved_path.try_exists()? {
                return Err(AppError::PathAlreadyExists(path));
            }

            std::fs::create_dir_all(resolved_path).map_err(Into::into)
        })
    }

    fn save(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let path = self.resolve(path);

        self.run_blocking(|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(path, bytes).map_err(Into::into)
        })
    }

    fn create_empty_file(&self, path: &FilePath) -> FutureResult<()> {
        let resolved_path = self.resolve(path);
        let path = path.clone();

        self.run_blocking(|| {
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

    fn delete_path(&self, path: &FilePath) -> FutureResult<()> {
        let root = self.id.project_path().as_path_buf();
        let resolved_path = self.resolve_exists(path);
        let path = path.clone();

        self.run_blocking(move || {
            let resolved_path = resolved_path?;
            if resolved_path.is_dir() {
                let mut files = Vec::new();
                let mut directories = vec![path];
                collect_entries(&root, &resolved_path, &mut files, &mut directories)?;

                std::fs::remove_dir_all(resolved_path)?;

                Ok(())
            } else if resolved_path.is_file() {
                std::fs::remove_file(resolved_path)?;

                Ok(())
            } else {
                Err(AppError::FileNotFound(path))
            }
        })
    }

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()> {
        let old_resolved_path = self.resolve_exists(old);
        let new_resolved_path = self.resolve(new);
        let old = old.clone();
        let new = new.clone();

        self.run_blocking(move || {
            if old == new {
                return Ok(());
            }

            let old_resolved_path = old_resolved_path?;
            if new_resolved_path.try_exists()? {
                return Err(AppError::PathAlreadyExists(new));
            }

            if let Some(parent) = new_resolved_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::rename(old_resolved_path, new_resolved_path)?;

            Ok(())
        })
    }
}

fn spawn_worker_thread(rx: std::sync::mpsc::Receiver<FileSystemJob>) {
    std::thread::Builder::new()
        .name("native-fs-worker".to_string())
        .spawn(move || {
            while let Ok(job) = rx.recv() {
                (job)();
            }
        })
        .expect("couldn't spawn thread");
}

fn spawn_blocking<T: Send + 'static>(
    send_jobs: std::sync::mpsc::Sender<FileSystemJob>,
    function: impl FnOnce() -> T + Send + 'static,
) -> AsyncJob<T> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let job = Box::new(move || {
        let result = function();
        tx.send(result).ok();
    });

    send_jobs.send(job).expect("the fs worker closed");

    AsyncJob::new(async move {
        rx.await
            .expect("function must complete unless the fs worker panics")
    })
}

fn collect_entries(
    root: &std::path::Path,
    current: &std::path::Path,
    files: &mut Vec<FilePath>,
    directories: &mut Vec<FilePath>,
) -> AppResult<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            match FilePath::from_relative_path(relative_path) {
                Ok(path) => directories.push(path),
                Err(err) => log::error!("Skipping invalid directory path {:?}: {}", path, err),
            }
            collect_entries(root, &path, files, directories)?;
        } else if file_type.is_file() {
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            match FilePath::from_relative_path(relative_path) {
                Ok(path) => files.push(path),
                Err(err) => log::error!("Skipping invalid file path {:?}: {}", path, err),
            }
        }
    }

    Ok(())
}
