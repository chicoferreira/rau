use crate::error::AppResult;
use crate::utils::async_job::AsyncJob;

#[cfg(not(target_arch = "wasm32"))]
pub use native::FileSystem;

#[cfg(target_arch = "wasm32")]
pub use wasm::FileSystem;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::path::PathBuf;

    use crate::{
        error::AppError,
        fs::{file_watcher::FileWatcher, identifier::ProjectIdentifier},
        project::paths::FilePath,
    };

    use super::*;

    #[derive(Clone)]
    pub struct FileSystem {}

    impl FileSystem {
        pub async fn new() -> AppResult<Self> {
            Ok(Self {})
        }

        pub fn create_file_watcher(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AppResult<FileWatcher> {
            FileWatcher::new(identifier.project_path().clone())
        }

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

        pub fn read(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<u8>>> {
            let path = self.resolve_exists(identifier, file_path);
            AsyncJob::new(async move {
                let path = path?;
                std::fs::read(&path).map_err(Into::into)
            })
        }

        pub fn read_to_string(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<String>> {
            let path = self.resolve_exists(identifier, file_path);
            AsyncJob::new(async move {
                let path = path?;
                std::fs::read_to_string(&path).map_err(Into::into)
            })
        }

        pub fn list_files(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let root = identifier.project_path().as_path_buf();

            AsyncJob::new(async move {
                let mut files = Vec::new();

                collect_files(&root, &root, &mut files)?;
                files.sort_by_key(|file| file.segments().to_vec());

                Ok(files)
            })
        }

        pub fn save(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
            content: Vec<u8>,
        ) -> AsyncJob<AppResult<()>> {
            let path = self.resolve(identifier, file_path);

            AsyncJob::new(async move {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                std::fs::write(path, content).map_err(Into::into)
            })
        }

        pub fn create_empty_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            self.save(identifier, file_path, vec![])
        }

        pub fn delete_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<Result<(), AppError>> {
            let path = self.resolve_exists(identifier, file_path);

            AsyncJob::new(async move {
                let path = path?;
                std::fs::remove_file(path).map_err(Into::into)
            })
        }
    }

    fn collect_files(
        root: &std::path::Path,
        current: &std::path::Path,
        files: &mut Vec<FilePath>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                collect_files(root, &path, files)?;
            } else if file_type.is_file() {
                let relative_path = path.strip_prefix(root).unwrap_or(&path);
                let relative_path = relative_path.to_string_lossy().replace('\\', "/");

                files.push(FilePath::from_relative_path(relative_path));
            }
        }

        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use indexed_db_futures::{
        Build, BuildPrimitive, database::Database, prelude::QuerySource, typed_array::Uint8Array,
    };
    use wasm_bindgen::JsValue;
    use web_sys::js_sys::Array;

    use crate::{
        error::AppError,
        fs::{file_watcher::FileWatcher, identifier::ProjectIdentifier},
        project::paths::FilePath,
    };

    use super::*;

    const DB_NAME: &str = "rau";
    const FILES_STORE: &str = "files";

    #[derive(Clone)]
    pub struct FileSystem {
        database: Database,
    }

    impl FileSystem {
        pub fn new() -> AsyncJob<AppResult<Self>> {
            AsyncJob::new(async move {
                let database = Self::open_database().await?;

                Ok(Self { database })
            })
        }

        async fn open_database() -> AppResult<Database> {
            let database = Database::open(DB_NAME).await?;

            if Self::has_files_store(&database) {
                return Ok(database);
            }

            let next_version = database.version() as u32 + 1;

            database.close();

            let database = Database::open(DB_NAME)
                .with_version(next_version)
                .with_on_upgrade_needed(|_event, database| {
                    if !Self::has_files_store(&database) {
                        database.create_object_store(FILES_STORE).build()?;
                    }

                    Ok(())
                })
                .await?;

            Ok(database)
        }

        fn has_files_store(database: &Database) -> bool {
            database
                .object_store_names()
                .any(|store_name| store_name == FILES_STORE)
        }

        fn file_key(identifier: &ProjectIdentifier, file_path: &FilePath) -> Array {
            std::iter::once(identifier.project_name())
                .chain(file_path.segments().iter().map(String::as_str))
                .map(JsValue::from_str)
                .collect()
        }

        fn project_file_path_from_key(
            identifier: &ProjectIdentifier,
            key: Array,
        ) -> Option<FilePath> {
            let key_project_name = key.get(0).as_string()?;

            if key_project_name != identifier.project_name() {
                return None;
            }

            let segments = (1..key.length())
                .map(|i| key.get(i).as_string())
                .collect::<Option<Vec<_>>>()?;

            Some(FilePath::new(segments))
        }

        pub fn create_file_watcher(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AppResult<FileWatcher> {
            let _ = identifier;
            FileWatcher::new()
        }

        pub fn save(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
            content: Vec<u8>,
        ) -> AsyncJob<AppResult<()>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                let transaction = database
                    .transaction(FILES_STORE)
                    .with_mode(web_sys::IdbTransactionMode::Readwrite)
                    .build()?;

                let store = transaction.object_store(FILES_STORE)?;
                let key = Self::file_key(&identifier, &file_path);

                let req = store.put(Uint8Array::from(content)).with_key(key).build()?;

                transaction.commit().await?;
                req.await?;

                Ok(())
            })
        }

        pub fn create_empty_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            self.save(identifier, file_path, vec![])
        }

        pub fn read(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<u8>>> {
            let identifier = identifier.clone();
            let database = self.database.clone();
            let file_path = file_path.clone();

            AsyncJob::new(Self::read_bytes(database, identifier, file_path))
        }

        pub fn exists(
            &self,
            identifier: ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<bool>> {
            let database = self.database.clone();
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                let transaction = database
                    .transaction(FILES_STORE)
                    .with_mode(web_sys::IdbTransactionMode::Readonly)
                    .build()?;

                let store = transaction.object_store(FILES_STORE)?;
                let key = Self::file_key(&identifier, &file_path);

                let file: Option<Uint8Array> = store.get(key).primitive()?.await?;

                Ok(file.is_some())
            })
        }

        pub fn read_to_string(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<String>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                let bytes = Self::read_bytes(database, identifier, file_path.clone()).await?;

                String::from_utf8(bytes).map_err(|_| AppError::FileNotValidUtf8(file_path))
            })
        }

        pub fn list_files(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let database = self.database.clone();
            let identifier = identifier.clone();

            AsyncJob::new(async move {
                let transaction = database
                    .transaction(FILES_STORE)
                    .with_mode(web_sys::IdbTransactionMode::Readonly)
                    .build()?;

                let store = transaction.object_store(FILES_STORE)?;

                let keys = store.get_all_keys().primitive()?.await?;
                let keys: Vec<Array> = keys.collect::<indexed_db_futures::Result<Vec<Array>>>()?;

                let mut files: Vec<_> = keys
                    .into_iter()
                    .filter_map(|key| Self::project_file_path_from_key(&identifier, key))
                    .collect();

                files.sort_by_key(|file| file.segments().to_vec());

                Ok(files)
            })
        }

        pub fn delete_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<Result<(), AppError>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let file_path = file_path.clone();
            AsyncJob::new(async move {
                let transaction = database
                    .transaction(FILES_STORE)
                    .with_mode(web_sys::IdbTransactionMode::Readwrite)
                    .build()?;

                let store = transaction.object_store(FILES_STORE)?;
                let key = Self::file_key(&identifier, &file_path);

                let delete_request = store.delete(key).build()?;
                transaction.commit().await?;
                delete_request.await?;

                Ok(())
            })
        }

        async fn read_bytes(
            database: Database,
            identifier: ProjectIdentifier,
            file_path: FilePath,
        ) -> AppResult<Vec<u8>> {
            let transaction = database
                .transaction(FILES_STORE)
                .with_mode(web_sys::IdbTransactionMode::Readonly)
                .build()?;

            let store = transaction.object_store(FILES_STORE)?;
            let key = Self::file_key(&identifier, &file_path);

            let file: Option<Uint8Array> = store.get(key).primitive()?.await?;

            match file {
                Some(data) => Ok(data.to_vec()),
                None => Err(AppError::FileNotFound(file_path)),
            }
        }
    }
}
