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
    ui::components::main_menu::recent_projects::RecentProjectEntry,
    utils::{async_job::AsyncJob, background_task},
};

type FileSystemJob = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone)]
pub struct AppFileSystem {
    send_jobs: std::sync::mpsc::Sender<FileSystemJob>,
}

#[derive(Clone)]
pub struct ProjectFileSystem {
    root: AbsolutePathBuf,
    send_jobs: std::sync::mpsc::Sender<FileSystemJob>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub recent_projects: Vec<RecentProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: AbsolutePathBuf,
    pub last_opened: chrono::DateTime<chrono::Utc>,
}

fn config_path() -> AppResult<PathBuf> {
    Ok(dirs::config_dir()
        .ok_or(AppError::ConfigDirectoryUnavailable)?
        .join("rau")
        .join("config.toml"))
}

impl AppConfig {
    fn read_or_default_if_empty() -> AppResult<Self> {
        let path = config_path()?;
        match std::fs::read_to_string(&path) {
            Ok(contents) => Ok(toml::from_str(&contents)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn read_or_default_if_error() -> AppConfig {
        match Self::read_or_default_if_empty() {
            Ok(config) => config,
            Err(err) => {
                log::error!("Failed to read app config: {err}");
                log::error!("Creating a new app config.");
                AppConfig::default()
            }
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
        self.recent_projects
            .retain(|recent| recent.path != project_path);
        self.recent_projects.push(RecentProject {
            path: project_path,
            last_opened: chrono::Utc::now(),
        });
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
    ) -> FutureResult<(super::ProjectFileSystem, FileWatcher)> {
        let send_jobs = self.send_jobs.clone();
        let root = id.project_path().clone();

        AsyncJob::new(async move {
            std::fs::create_dir_all(&root)?; // File watcher requires the directory to exist

            let file_watcher = FileWatcher::os(root.clone())?;
            let file_system = ProjectFileSystem { root, send_jobs };

            Ok((super::ProjectFileSystem::Native(file_system), file_watcher))
        })
    }

    fn recent_projects(&self) -> FutureResult<Vec<RecentProjectEntry>> {
        spawn_blocking(self.send_jobs.clone(), move || {
            let config = AppConfig::read_or_default_if_empty()?;
            let mut seen = HashSet::new();
            let mut projects = Vec::new();

            for RecentProject { path, last_opened } in config.recent_projects {
                match ProjectIdentifier::extract_identifier(path) {
                    Ok(id) => {
                        let project_path = id.project_path().as_path_buf();
                        if seen.insert(project_path) {
                            projects.push(RecentProjectEntry { id, last_opened });
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

    fn ensure_project_can_be_created(&self, id: ProjectIdentifier) -> FutureResult<()> {
        spawn_blocking(self.send_jobs.clone(), move || {
            let project_path = id.project_path().as_path_buf();
            let project_json_path = project_path.join(FilePath::project_json().to_string());

            if project_json_path.try_exists()? {
                return Err(AppError::ProjectJsonAlreadyExists(project_path));
            }

            Ok(())
        })
    }

    fn remember_project(&self, id: ProjectIdentifier) -> FutureResult<()> {
        let project_path = id.project_path().clone();

        spawn_blocking(self.send_jobs.clone(), move || {
            let mut config = AppConfig::read_or_default_if_error();
            config.remember_project_path(project_path);
            config.save()
        })
    }

    fn remove_recent_project(&self, id: ProjectIdentifier) -> FutureResult<()> {
        let project_path = id.project_path().clone();

        spawn_blocking(self.send_jobs.clone(), move || {
            let mut config = AppConfig::read_or_default_if_error();
            config
                .recent_projects
                .retain(|recent| recent.path != project_path);
            config.save()
        })
    }
}

impl ProjectFileSystem {
    pub fn root(&self) -> &AbsolutePathBuf {
        &self.root
    }

    fn resolve(&self, file_path: &FilePath) -> PathBuf {
        let mut path_buf = self.root.as_path_buf();
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

    fn exists(&self, path: &FilePath) -> FutureResult<bool> {
        let path = self.resolve(path);

        self.run_blocking(move || path.try_exists().map_err(Into::into))
    }

    fn list_entries(&self) -> FutureResult<FileSystemEntries> {
        let root = self.root.as_path_buf();
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

    fn write(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let path = self.resolve(path);

        self.run_blocking(|| {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(path, bytes).map_err(Into::into)
        })
    }

    fn delete_path(&self, path: &FilePath) -> FutureResult<()> {
        let root = self.root.as_path_buf();
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
    let (sender, task) = background_task::channel();
    let job = Box::new(move || sender.send(function()));

    send_jobs.send(job).expect("the fs worker closed");

    task
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
