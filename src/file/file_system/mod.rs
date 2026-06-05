mod ephemeral;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

use enum_dispatch::enum_dispatch;

use crate::{
    error::{AppError, AppResult},
    file::{
        file_system::ephemeral::EphemeralFileSystem,
        file_watcher::FileWatcher,
        identifier::{ProjectIdentifier, ProjectSource},
    },
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

#[cfg(not(target_arch = "wasm32"))]
use native::AppFileSystem as BackendFileSystem;
#[cfg(target_arch = "wasm32")]
use wasm::AppFileSystem as BackendFileSystem;

#[derive(Clone)]
#[enum_dispatch(ProjectFileSystemTrait)]
pub enum ProjectFileSystem {
    Ephemeral(EphemeralFileSystem),
    #[cfg(not(target_arch = "wasm32"))]
    Native(native::ProjectFileSystem),
    #[cfg(target_arch = "wasm32")]
    IndexedDb(wasm::ProjectFileSystem),
}

#[derive(Debug, Clone, Default)]
pub struct FileSystemEntries {
    pub files: Vec<FilePath>,
    pub directories: Vec<FilePath>,
}

pub type FutureResult<T> = AsyncJob<AppResult<T>>;

pub trait AppFileSystemTrait: Clone + Sized {
    fn open() -> FutureResult<Self>;

    fn mount_project(
        &self,
        id: ProjectIdentifier,
    ) -> FutureResult<(ProjectFileSystem, FileWatcher)>;

    fn recent_projects(&self) -> FutureResult<Vec<ProjectIdentifier>>;

    fn ensure_project_can_be_created(&self, id: ProjectIdentifier) -> FutureResult<()>;

    fn remember_project(&self, id: ProjectIdentifier) -> FutureResult<()>;

    fn remove_recent_project(&self, id: ProjectIdentifier) -> FutureResult<()>;
}

#[enum_dispatch]
pub trait ProjectFileSystemTrait: Clone + Sized {
    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>>;

    fn exists(&self, path: &FilePath) -> FutureResult<bool>;

    fn list_entries(&self) -> FutureResult<FileSystemEntries>;

    fn create_directory(&self, path: &FilePath) -> FutureResult<()>;

    fn write(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()>;

    fn delete_path(&self, path: &FilePath) -> FutureResult<()>;

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()>;
}

impl ProjectFileSystem {
    pub fn read_to_string(&self, path: &FilePath) -> FutureResult<String> {
        let (file_system, path) = (self.clone(), path.clone());
        AsyncJob::new(async move {
            let bytes = file_system.read(&path).await?;
            String::from_utf8(bytes).map_err(|_| AppError::FileNotValidUtf8(path))
        })
    }

    pub fn create_empty_file(&self, path: &FilePath) -> FutureResult<()> {
        let (file_system, path) = (self.clone(), path.clone());
        AsyncJob::new(async move {
            if file_system.exists(&path).await? {
                return Err(AppError::PathAlreadyExists(path));
            }

            file_system.write(&path, Vec::new()).await
        })
    }
}

#[derive(Clone)]
pub struct AppFileSystem {
    backend: BackendFileSystem,
}

impl AppFileSystem {
    pub fn open() -> FutureResult<Self> {
        let backend = BackendFileSystem::open();
        AsyncJob::new(async move {
            Ok(Self {
                backend: backend.await?,
            })
        })
    }

    pub fn mount_project(
        &self,
        source: ProjectSource,
    ) -> FutureResult<(ProjectFileSystem, FileWatcher)> {
        match source {
            ProjectSource::Ephemeral { .. } => AsyncJob::new(async move {
                let (change_sender, file_watcher) = FileWatcher::manual();
                let file_system =
                    ProjectFileSystem::Ephemeral(EphemeralFileSystem::new(change_sender));
                Ok((file_system, file_watcher))
            }),
            ProjectSource::Persistent(id) => self.backend.mount_project(id),
        }
    }

    pub fn recent_projects(&self) -> FutureResult<Vec<ProjectIdentifier>> {
        self.backend.recent_projects()
    }

    pub fn ensure_project_can_be_created(&self, source: ProjectSource) -> FutureResult<()> {
        match source {
            ProjectSource::Ephemeral { .. } => AsyncJob::new(async move { Ok(()) }),
            ProjectSource::Persistent(id) => self.backend.ensure_project_can_be_created(id),
        }
    }

    pub fn remember_project(&self, source: ProjectSource) -> FutureResult<()> {
        match source {
            ProjectSource::Ephemeral { .. } => AsyncJob::new(async move { Ok(()) }),
            ProjectSource::Persistent(id) => self.backend.remember_project(id),
        }
    }

    pub fn remove_recent_project(&self, source: ProjectSource) -> FutureResult<()> {
        match source {
            ProjectSource::Ephemeral { .. } => AsyncJob::new(async move { Ok(()) }),
            ProjectSource::Persistent(id) => self.backend.remove_recent_project(id),
        }
    }
}
