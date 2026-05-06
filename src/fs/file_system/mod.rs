#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod wasm;

use crate::{
    error::AppResult,
    fs::{file_watcher::FileWatcher, identifier::ProjectIdentifier},
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

#[allow(dead_code)]
pub trait FileSystemTrait: Clone + Sized {
    fn new() -> FutureResult<Self>;

    fn create_file_watcher(&self, id: &ProjectIdentifier) -> AppResult<FileWatcher>;

    fn read(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<u8>>;

    fn read_to_string(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<String>;

    fn list_entries(&self, id: &ProjectIdentifier) -> FutureResult<FileSystemEntries>;

    fn create_directory(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()>;

    fn save(&self, id: &ProjectIdentifier, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()>;

    fn create_empty_file(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()>;

    fn delete_path(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<FilePath>>;

    fn move_path(
        &self,
        id: &ProjectIdentifier,
        old: &FilePath,
        new: &FilePath,
    ) -> FutureResult<Vec<FilePath>>;
}
