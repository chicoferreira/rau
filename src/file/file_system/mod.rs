mod ephemeral;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

use enum_dispatch::enum_dispatch;

use crate::{
    error::{AppError, AppResult},
    file::{file_watcher::FileWatcher, identifier::ProjectIdentifier},
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

pub use ephemeral::EphemeralFileSystem;
#[cfg(not(target_arch = "wasm32"))]
pub use native::AppFileSystem;
#[cfg(target_arch = "wasm32")]
pub use wasm::AppFileSystem;

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
    pub fn ephemeral() -> Self {
        Self::Ephemeral(EphemeralFileSystem::default())
    }

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
