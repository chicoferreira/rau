#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

use crate::{
    error::AppResult,
    file::{file_watcher::FileWatcher, identifier::ProjectIdentifier},
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

#[cfg(not(target_arch = "wasm32"))]
pub use native::{AppFileSystem, ProjectFileSystem};
#[cfg(target_arch = "wasm32")]
pub use wasm::{AppFileSystem, ProjectFileSystem};

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

    fn remember_project(&self, id: ProjectIdentifier) -> FutureResult<()>;

    fn remove_recent_project(&self, id: ProjectIdentifier) -> FutureResult<()>;
}

pub trait ProjectFileSystemTrait: Clone + Sized {
    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>>;

    fn read_to_string(&self, path: &FilePath) -> FutureResult<String>;

    fn exists(&self, path: &FilePath) -> FutureResult<bool>;

    fn list_entries(&self) -> FutureResult<FileSystemEntries>;

    fn create_directory(&self, path: &FilePath) -> FutureResult<()>;

    fn save(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()>;

    fn create_empty_file(&self, path: &FilePath) -> FutureResult<()>;

    fn delete_path(&self, path: &FilePath) -> FutureResult<()>;

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()>;
}
