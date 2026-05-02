use std::fmt::Display;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::{error::AppResult, utils::pollable_future::PollableFuture};

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileSystem;

#[cfg(target_arch = "wasm32")]
pub use wasm::FileSystem;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFilePath {
    path: String,
}

impl ProjectFilePath {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }
}

impl Display for ProjectFilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use crate::error::AppError;
    use crate::fs::{absolute::AbsolutePathBuf, file_watcher::FileWatcher};

    use super::*;

    #[derive(Clone)]
    pub struct FileSystem {
        root: AbsolutePathBuf,
    }

    impl FileSystem {
        pub fn new(root: AbsolutePathBuf) -> AppResult<Self> {
            Ok(Self { root })
        }

        pub fn create_file_watcher(&self) -> AppResult<FileWatcher> {
            FileWatcher::new(self.root.clone())
        }

        pub fn resolve(&self, path: &ProjectFilePath) -> AppResult<PathBuf> {
            let path_buf = self.root.as_ref().join(&path.path); // TODO: error if path is invalid

            if !path_buf.exists() {
                return Err(AppError::FileNotFound(path.clone()));
            }

            Ok(path_buf)
        }

        pub fn read(&self, file_path: &ProjectFilePath) -> PollableFuture<AppResult<Vec<u8>>> {
            let path = self.resolve(file_path);
            PollableFuture::new(async move {
                let path = path?;
                std::fs::read(&path).map_err(Into::into)
            })
        }

        pub fn read_to_string(
            &self,
            file_path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<String>> {
            let path = self.resolve(file_path);
            PollableFuture::new(async move {
                let path = path?;
                std::fs::read_to_string(&path).map_err(Into::into)
            })
        }

        pub fn list_files(
            &self,
            _path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<Vec<ProjectFilePath>>> {
            PollableFuture::new(async move { todo!() })
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use indexed_db_futures::{
        Build, BuildPrimitive, database::Database, prelude::QuerySource, typed_array::Uint8Array,
    };

    use crate::{error::AppError, fs::file_watcher::FileWatcher};

    use super::*;

    #[derive(Clone)]
    pub struct FileSystem {
        project_name: String,
        database: Database,
    }

    impl FileSystem {
        pub fn new(project_name: String) -> PollableFuture<AppResult<Self>> {
            PollableFuture::new(async move {
                let database = Database::open("rau").await?;
                Ok(Self {
                    project_name,
                    database,
                })
            })
        }

        pub fn create_file_watcher(&self) -> AppResult<FileWatcher> {
            FileWatcher::new()
        }

        pub fn read(&self, file_path: &ProjectFilePath) -> PollableFuture<AppResult<Vec<u8>>> {
            let database = self.database.clone();
            let project_name = self.project_name.clone();
            let file_path = file_path.clone();

            PollableFuture::new(Self::read_bytes(database, project_name, file_path))
        }

        pub fn read_to_string(
            &self,
            file_path: &ProjectFilePath,
        ) -> PollableFuture<AppResult<String>> {
            let database = self.database.clone();
            let project_name = self.project_name.clone();
            let file_path = file_path.clone();

            PollableFuture::new(async move {
                let bytes = Self::read_bytes(database, project_name, file_path.clone()).await?;
                let text = String::from_utf8(bytes);
                let text = text.map_err(|_| AppError::FileNotValidUtf8(file_path))?;

                Ok(text)
            })
        }

        pub fn list_files(&self) -> PollableFuture<AppResult<Vec<ProjectFilePath>>> {
            let database = self.database.clone();
            let project_name = self.project_name.clone();

            PollableFuture::new(async move {
                let transaction = database
                    .transaction(&project_name)
                    .with_mode(web_sys::IdbTransactionMode::Readonly)
                    .build()?;

                let store = transaction.object_store(&project_name)?;

                let files: Vec<String> = store
                    .get_all_keys()
                    .primitive()?
                    .await?
                    .collect::<Result<Vec<String>, indexed_db_futures::error::Error>>()?;

                let files = files.into_iter().map(ProjectFilePath::new).collect();

                Ok(files)
            })
        }

        async fn read_bytes(
            database: Database,
            project_name: String,
            file_path: ProjectFilePath,
        ) -> AppResult<Vec<u8>> {
            let transaction = database
                .transaction(&project_name)
                .with_mode(web_sys::IdbTransactionMode::Readonly)
                .build()?;

            let store = transaction.object_store(&project_name)?;

            let file: Option<Uint8Array> = store.get(&file_path.path).primitive()?.await?;

            match file {
                Some(data) => Ok(data.to_vec()),
                None => Err(AppError::FileNotFound(file_path)),
            }
        }
    }
}
