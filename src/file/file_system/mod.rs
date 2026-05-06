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
pub use native::FileSystem;
#[cfg(target_arch = "wasm32")]
pub use wasm::FileSystem;

#[derive(Debug, Clone, Default)]
pub struct FileSystemEntries {
    pub files: Vec<FilePath>,
    pub directories: Vec<FilePath>,
}

type FutureResult<T> = AsyncJob<AppResult<T>>;

pub trait FileSystemTrait: Clone + Sized {
    fn mount(id: ProjectIdentifier) -> FutureResult<(Self, FileWatcher)>;

    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>>;

    fn read_to_string(&self, path: &FilePath) -> FutureResult<String>;

    fn list_entries(&self) -> FutureResult<FileSystemEntries>;

    fn create_directory(&self, path: &FilePath) -> FutureResult<()>;

    #[allow(dead_code)]
    fn save(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()>;

    fn create_empty_file(&self, path: &FilePath) -> FutureResult<()>;

    fn delete_path(&self, path: &FilePath) -> FutureResult<()>;

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()>;
}
