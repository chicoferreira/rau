use crate::error::AppResult;
use crate::utils::async_job::AsyncJob;

#[derive(Debug, Clone, Default)]
pub struct FileSystemEntries {
    pub files: Vec<crate::project::paths::FilePath>,
    pub directories: Vec<crate::project::paths::FilePath>,
}

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

        pub fn list_entries(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AsyncJob<AppResult<FileSystemEntries>> {
            let root = identifier.project_path().as_path_buf();

            AsyncJob::new(async move {
                let mut files = Vec::new();
                let mut directories = Vec::new();

                collect_entries(&root, &root, &mut files, &mut directories)?;
                files.sort_by_key(|file| file.segments().to_vec());
                directories.sort_by_key(|directory| directory.segments().to_vec());

                Ok(FileSystemEntries { files, directories })
            })
        }

        pub fn create_directory(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            let path = self.resolve(identifier, file_path);
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                if path.try_exists()? {
                    return Err(AppError::PathAlreadyExists(file_path));
                }

                std::fs::create_dir_all(path).map_err(Into::into)
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
            let path = self.resolve(identifier, file_path);
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                if path.try_exists()? {
                    return Err(AppError::PathAlreadyExists(file_path));
                }

                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    Ok(_) => Ok(()),
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                        Err(AppError::PathAlreadyExists(file_path))
                    }
                    Err(err) => Err(err.into()),
                }
            })
        }

        pub fn delete_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            let path = self.resolve_exists(identifier, file_path);

            AsyncJob::new(async move {
                let path = path?;
                std::fs::remove_file(path).map_err(Into::into)
            })
        }

        pub fn delete_directory(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let root = identifier.project_path().as_path_buf();
            let path = self.resolve_exists(identifier, file_path);
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                let path = path?;
                let mut files = Vec::new();
                let mut directories = vec![file_path];
                collect_entries(&root, &path, &mut files, &mut directories)?;

                std::fs::remove_dir_all(path)?;

                Ok(directories.into_iter().chain(files).collect())
            })
        }

        pub fn move_path(
            &self,
            identifier: &ProjectIdentifier,
            old_path: &FilePath,
            new_path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let root = identifier.project_path().as_path_buf();
            let old_resolved_path = self.resolve_exists(identifier, old_path);
            let new_resolved_path = self.resolve(identifier, new_path);
            let old_path = old_path.clone();
            let new_path = new_path.clone();

            AsyncJob::new(async move {
                if old_path == new_path {
                    return Ok(Vec::new());
                }

                let old_resolved_path = old_resolved_path?;
                if new_resolved_path.try_exists()? {
                    return Err(AppError::PathAlreadyExists(new_path));
                }

                let mut changed_paths = Vec::new();
                if old_resolved_path.is_dir() {
                    if new_path.starts_with(&old_path) {
                        return Err(AppError::PathAlreadyExists(new_path));
                    }

                    let mut files = Vec::new();
                    let mut directories = vec![old_path.clone()];
                    collect_entries(&root, &old_resolved_path, &mut files, &mut directories)?;

                    changed_paths.extend(directories.iter().cloned());
                    changed_paths.extend(files.iter().cloned());
                    changed_paths.extend(
                        directories
                            .iter()
                            .chain(files.iter())
                            .filter_map(|path| path.replace_prefix(&old_path, &new_path)),
                    );
                } else {
                    changed_paths.push(old_path);
                    changed_paths.push(new_path.clone());
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
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use indexed_db_futures::{
        Build, BuildPrimitive,
        database::Database,
        future::BasicRequest,
        object_store::ObjectStore,
        prelude::QuerySource,
        transaction::{Transaction, TransactionMode, TransactionRef},
        typed_array::Uint8Array,
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
    const DIRECTORIES_STORE: &str = "directories";

    // Directory invariant for new writes:
    // - the project root is implicit and is never stored;
    // - every non-root parent directory of a newly saved file is materialized in DIRECTORIES_STORE;
    // - creating a directory materializes that directory and its non-root ancestors;

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

            if Self::has_files_store(&database) && Self::has_directories_store(&database) {
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
                    if !Self::has_directories_store(&database) {
                        database.create_object_store(DIRECTORIES_STORE).build()?;
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

        fn has_directories_store(database: &Database) -> bool {
            database
                .object_store_names()
                .any(|store_name| store_name == DIRECTORIES_STORE)
        }

        fn key(identifier: &ProjectIdentifier, file_path: &FilePath) -> Array {
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

        fn files_transaction(
            database: &Database,
            mode: TransactionMode,
        ) -> AppResult<Transaction<'_>> {
            database
                .transaction(FILES_STORE)
                .with_mode(mode)
                .build()
                .map_err(Into::into)
        }

        fn entries_transaction(
            database: &Database,
            mode: TransactionMode,
        ) -> AppResult<Transaction<'_>> {
            database
                .transaction([FILES_STORE, DIRECTORIES_STORE])
                .with_mode(mode)
                .build()
                .map_err(Into::into)
        }

        fn files_store<'a>(transaction: &'a TransactionRef<'a>) -> AppResult<ObjectStore<'a>> {
            transaction.object_store(FILES_STORE).map_err(Into::into)
        }

        fn directories_store<'a>(
            transaction: &'a TransactionRef<'a>,
        ) -> AppResult<ObjectStore<'a>> {
            transaction
                .object_store(DIRECTORIES_STORE)
                .map_err(Into::into)
        }

        async fn entry_exists_in_transaction(
            transaction: &TransactionRef<'_>,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AppResult<bool> {
            let files_store = Self::files_store(transaction)?;
            let directories_store = Self::directories_store(transaction)?;

            let file_count = files_store
                .count()
                .with_query(Self::key(identifier, file_path))
                .primitive()?;
            let directory_count = directories_store
                .count()
                .with_query(Self::key(identifier, file_path))
                .primitive()?;

            let (file_count, directory_count) =
                futures_lite::future::try_zip(file_count, directory_count).await?;

            Ok(file_count > 0 || directory_count > 0)
        }

        fn ensure_directories(
            directories_store: &ObjectStore<'_>,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AppResult<Vec<BasicRequest<Array>>> {
            file_path
                .ancestors_inclusive()
                .into_iter()
                .map(|directory| Self::key(identifier, &directory))
                .map(|key| Ok(directories_store.put(JsValue::TRUE).with_key(key).build()?))
                .collect()
        }

        fn ensure_parent_directories(
            directories_store: &ObjectStore<'_>,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AppResult<Vec<BasicRequest<Array>>> {
            let Some(parent) = file_path.parent() else {
                return Ok(Vec::new());
            };

            Self::ensure_directories(directories_store, identifier, &parent)
        }

        async fn read_bytes_from_store(
            store: ObjectStore<'_>,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AppResult<Vec<u8>> {
            let key = Self::key(identifier, file_path);
            let file: Option<Uint8Array> = store.get(key).primitive()?.await?;

            match file {
                Some(data) => Ok(data.to_vec()),
                None => Err(AppError::FileNotFound(file_path.clone())),
            }
        }

        async fn list_store_paths(
            identifier: &ProjectIdentifier,
            store: ObjectStore<'_>,
        ) -> AppResult<Vec<FilePath>> {
            Ok(store
                .get_all_keys()
                .primitive()?
                .await?
                .collect::<indexed_db_futures::Result<Vec<Array>>>()?
                .into_iter()
                .filter_map(|key| Self::project_file_path_from_key(identifier, key))
                .filter(|path| !path.segments().is_empty())
                .collect())
        }

        async fn keyed_paths_under_path(
            identifier: &ProjectIdentifier,
            path: &FilePath,
            store: ObjectStore<'_>,
        ) -> AppResult<Vec<(Array, FilePath)>> {
            Ok(store
                .get_all_keys()
                .primitive()?
                .await?
                .collect::<indexed_db_futures::Result<Vec<Array>>>()?
                .into_iter()
                .filter_map(|key| {
                    Self::project_file_path_from_key(identifier, key.clone())
                        .filter(|file_path| file_path.starts_with(path))
                        .map(|file_path| (key, file_path))
                })
                .collect())
        }

        async fn file_entries_under_path(
            identifier: &ProjectIdentifier,
            path: &FilePath,
            store: ObjectStore<'_>,
        ) -> AppResult<Vec<(Array, FilePath, Vec<u8>)>> {
            let keys = store
                .get_all_keys()
                .primitive()?
                .await?
                .collect::<indexed_db_futures::Result<Vec<Array>>>()?;
            let mut entries = Vec::new();

            for key in keys {
                let Some(file_path) = Self::project_file_path_from_key(identifier, key.clone())
                    .filter(|file_path| file_path.starts_with(path))
                else {
                    continue;
                };

                let file: Option<Uint8Array> = store.get(key.clone()).primitive()?.await?;
                let Some(file) = file else {
                    return Err(AppError::FileNotFound(file_path));
                };

                entries.push((key, file_path, file.to_vec()));
            }

            Ok(entries)
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
                let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                Self::ensure_parent_directories(&directories_store, &identifier, &file_path)?;

                let key = Self::key(&identifier, &file_path);

                files_store
                    .put(Uint8Array::from(content))
                    .with_key(key)
                    .build()?;

                transaction.commit().await?;

                Ok(())
            })
        }

        pub fn create_directory(
            &self,
            identifier: &ProjectIdentifier,
            path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let path = path.clone();

            AsyncJob::new(async move {
                let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                if Self::entry_exists_in_transaction(&transaction, &identifier, &path).await? {
                    return Err(AppError::PathAlreadyExists(path));
                }

                let directories_store = Self::directories_store(&transaction)?;

                Self::ensure_directories(&directories_store, &identifier, &path)?;

                transaction.commit().await?;

                Ok(())
            })
        }

        pub fn create_empty_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let file_path = file_path.clone();

            AsyncJob::new(async move {
                let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                if Self::entry_exists_in_transaction(&transaction, &identifier, &file_path).await? {
                    return Err(AppError::PathAlreadyExists(file_path));
                }

                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                Self::ensure_parent_directories(&directories_store, &identifier, &file_path)?;

                let key = Self::key(&identifier, &file_path);
                files_store
                    .add(Uint8Array::from(Vec::new()))
                    .with_key(key)
                    .build()?;

                transaction.commit().await?;

                Ok(())
            })
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

        pub fn list_entries(
            &self,
            identifier: &ProjectIdentifier,
        ) -> AsyncJob<AppResult<FileSystemEntries>> {
            let database = self.database.clone();
            let id = identifier.clone();

            AsyncJob::new(async move {
                let transaction = Self::entries_transaction(&database, TransactionMode::Readonly)?;
                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                let (files, directories) = futures_lite::future::try_zip(
                    Self::list_store_paths(&id, files_store),
                    Self::list_store_paths(&id, directories_store),
                )
                .await?;

                Ok(FileSystemEntries { files, directories })
            })
        }

        pub fn delete_file(
            &self,
            identifier: &ProjectIdentifier,
            file_path: &FilePath,
        ) -> AsyncJob<AppResult<()>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let file_path = file_path.clone();
            AsyncJob::new(async move {
                let transaction = Self::files_transaction(&database, TransactionMode::Readwrite)?;
                let store = Self::files_store(&transaction)?;
                let key = Self::key(&identifier, &file_path);

                store.delete(key).build()?;
                transaction.commit().await?;

                Ok(())
            })
        }

        pub fn delete_directory(
            &self,
            identifier: &ProjectIdentifier,
            path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let path = path.clone();

            AsyncJob::new(async move {
                let transaction = Self::entries_transaction(&database, TransactionMode::Readonly)?;
                if !Self::entry_exists_in_transaction(&transaction, &identifier, &path).await? {
                    return Err(AppError::FileNotFound(path));
                }

                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                let (file_entries, directory_entries) = futures_lite::future::try_zip(
                    Self::keyed_paths_under_path(&identifier, &path, files_store),
                    Self::keyed_paths_under_path(&identifier, &path, directories_store),
                )
                .await?;

                drop(transaction);

                let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                let mut changed_paths =
                    Vec::with_capacity(file_entries.len() + directory_entries.len());
                changed_paths.extend(file_entries.iter().map(|(_, path)| path.clone()));
                changed_paths.extend(directory_entries.iter().map(|(_, path)| path.clone()));

                for (key, _) in file_entries {
                    files_store.delete(key).build()?;
                }
                for (key, _) in directory_entries {
                    directories_store.delete(key).build()?;
                }

                transaction.commit().await?;

                Ok(changed_paths)
            })
        }

        pub fn move_path(
            &self,
            identifier: &ProjectIdentifier,
            old_path: &FilePath,
            new_path: &FilePath,
        ) -> AsyncJob<AppResult<Vec<FilePath>>> {
            let database = self.database.clone();
            let identifier = identifier.clone();
            let old_path = old_path.clone();
            let new_path = new_path.clone();

            AsyncJob::new(async move {
                if old_path == new_path {
                    return Ok(Vec::new());
                }

                if new_path.starts_with(&old_path) {
                    return Err(AppError::PathAlreadyExists(new_path));
                }

                let transaction = Self::entries_transaction(&database, TransactionMode::Readonly)?;

                if Self::entry_exists_in_transaction(&transaction, &identifier, &new_path).await? {
                    return Err(AppError::PathAlreadyExists(new_path));
                }

                let files_store = Self::files_store(&transaction)?;
                match Self::read_bytes_from_store(files_store, &identifier, &old_path).await {
                    Ok(bytes) => {
                        drop(transaction);

                        let transaction =
                            Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                        let files_store = Self::files_store(&transaction)?;
                        let directories_store = Self::directories_store(&transaction)?;

                        Self::ensure_parent_directories(
                            &directories_store,
                            &identifier,
                            &new_path,
                        )?;

                        files_store
                            .add(Uint8Array::from(bytes))
                            .with_key(Self::key(&identifier, &new_path))
                            .build()?;
                        files_store
                            .delete(Self::key(&identifier, &old_path))
                            .build()?;

                        transaction.commit().await?;

                        return Ok(vec![old_path, new_path]);
                    }
                    Err(AppError::FileNotFound(_)) => {}
                    Err(error) => return Err(error),
                }

                if !Self::entry_exists_in_transaction(&transaction, &identifier, &old_path).await? {
                    return Err(AppError::FileNotFound(old_path));
                }

                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                let (file_entries, directory_entries) = futures_lite::future::try_zip(
                    Self::file_entries_under_path(&identifier, &old_path, files_store),
                    Self::keyed_paths_under_path(&identifier, &old_path, directories_store),
                )
                .await?;

                let mut changed_paths =
                    Vec::with_capacity((file_entries.len() + directory_entries.len()) * 2);

                for (_, directory_path) in &directory_entries {
                    changed_paths.push(directory_path.clone());
                }
                for (_, file_path, _) in &file_entries {
                    changed_paths.push(file_path.clone());
                }

                let mut moved_directories = Vec::with_capacity(directory_entries.len());
                for (key, directory_path) in directory_entries {
                    let moved_path = directory_path
                        .replace_prefix(&old_path, &new_path)
                        .expect("directory entry was collected from the old path");

                    changed_paths.push(moved_path.clone());
                    moved_directories.push((key, moved_path));
                }

                let mut moved_files = Vec::with_capacity(file_entries.len());
                for (key, file_path, bytes) in file_entries {
                    let moved_path = file_path
                        .replace_prefix(&old_path, &new_path)
                        .expect("file entry was collected from the old path");

                    changed_paths.push(moved_path.clone());
                    moved_files.push((key, moved_path, bytes));
                }

                drop(transaction);

                let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                let files_store = Self::files_store(&transaction)?;
                let directories_store = Self::directories_store(&transaction)?;

                Self::ensure_parent_directories(&directories_store, &identifier, &new_path)?;

                for (_, moved_path) in &moved_directories {
                    directories_store
                        .put(JsValue::TRUE)
                        .with_key(Self::key(&identifier, moved_path))
                        .build()?;
                }
                for (_, moved_path, bytes) in &moved_files {
                    files_store
                        .add(Uint8Array::from(bytes.clone()))
                        .with_key(Self::key(&identifier, moved_path))
                        .build()?;
                }
                for (key, _) in moved_directories {
                    directories_store.delete(key).build()?;
                }
                for (key, _, _) in moved_files {
                    files_store.delete(key).build()?;
                }

                transaction.commit().await?;

                Ok(changed_paths)
            })
        }

        async fn read_bytes(
            database: Database,
            identifier: ProjectIdentifier,
            file_path: FilePath,
        ) -> AppResult<Vec<u8>> {
            let transaction = Self::files_transaction(&database, TransactionMode::Readonly)?;
            let store = Self::files_store(&transaction)?;
            Self::read_bytes_from_store(store, &identifier, &file_path).await
        }
    }
}
