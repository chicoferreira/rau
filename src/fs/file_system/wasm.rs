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
    error::{AppError, AppResult},
    fs::{
        file_system::{FileSystemEntries, FileSystemTrait, FutureResult},
        file_watcher::FileWatcher,
        identifier::ProjectIdentifier,
    },
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

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

    fn project_file_path_from_key(identifier: &ProjectIdentifier, key: Array) -> Option<FilePath> {
        let key_project_name = key.get(0).as_string()?;

        if key_project_name != identifier.project_name() {
            return None;
        }

        let segments = (1..key.length())
            .map(|i| key.get(i).as_string())
            .collect::<Option<Vec<_>>>()?;

        Some(FilePath::new(segments))
    }

    fn files_transaction(database: &Database, mode: TransactionMode) -> AppResult<Transaction<'_>> {
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

    fn directories_store<'a>(transaction: &'a TransactionRef<'a>) -> AppResult<ObjectStore<'a>> {
        transaction
            .object_store(DIRECTORIES_STORE)
            .map_err(Into::into)
    }

    async fn entry_exists(
        database: &Database,
        identifier: &ProjectIdentifier,
        file_path: &FilePath,
    ) -> AppResult<bool> {
        let transaction = Self::entries_transaction(database, TransactionMode::Readonly)?;
        let files_store = Self::files_store(&transaction)?;
        let directories_store = Self::directories_store(&transaction)?;

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
        database: &Database,
        identifier: &ProjectIdentifier,
        path: &FilePath,
    ) -> AppResult<Vec<(Array, FilePath, Vec<u8>)>> {
        let transaction = Self::files_transaction(database, TransactionMode::Readonly)?;
        let store = Self::files_store(&transaction)?;
        let keys = store
            .get_all_keys()
            .primitive()?
            .await?
            .collect::<indexed_db_futures::Result<Vec<Array>>>()?;
        drop(transaction);

        let mut entries = Vec::new();

        for key in keys {
            let Some(file_path) = Self::project_file_path_from_key(identifier, key.clone())
                .filter(|file_path| file_path.starts_with(path))
            else {
                continue;
            };

            let bytes =
                Self::read_bytes(database.clone(), identifier.clone(), file_path.clone()).await?;
            entries.push((key, file_path, bytes));
        }

        Ok(entries)
    }

    async fn read_bytes(
        database: Database,
        identifier: ProjectIdentifier,
        file_path: FilePath,
    ) -> AppResult<Vec<u8>> {
        let transaction = Self::files_transaction(&database, TransactionMode::Readonly)?;
        let store = Self::files_store(&transaction)?;
        let key = Self::key(&identifier, &file_path);
        let file: Option<Uint8Array> = store.get(key).primitive()?.await?;

        match file {
            Some(data) => Ok(data.to_vec()),
            None => Err(AppError::FileNotFound(file_path)),
        }
    }
}

impl FileSystemTrait for FileSystem {
    fn new() -> FutureResult<Self> {
        AsyncJob::new(async move {
            let database = Self::open_database().await?;

            Ok(Self { database })
        })
    }

    fn create_file_watcher(&self, id: &ProjectIdentifier) -> AppResult<FileWatcher> {
        let _ = id;
        FileWatcher::new()
    }

    fn save(&self, id: &ProjectIdentifier, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let database = self.database.clone();
        let id = id.clone();
        let path = path.clone();

        AsyncJob::new(async move {
            let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
            let files_store = Self::files_store(&transaction)?;
            let directories_store = Self::directories_store(&transaction)?;

            Self::ensure_parent_directories(&directories_store, &id, &path)?;

            let key = Self::key(&id, &path);

            files_store
                .put(Uint8Array::from(bytes))
                .with_key(key)
                .build()?;

            transaction.commit().await?;

            Ok(())
        })
    }

    fn create_directory(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()> {
        let database = self.database.clone();
        let id = id.clone();
        let path = path.clone();

        AsyncJob::new(async move {
            if Self::entry_exists(&database, &id, &path).await? {
                return Err(AppError::PathAlreadyExists(path));
            }

            let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
            let directories_store = Self::directories_store(&transaction)?;

            Self::ensure_directories(&directories_store, &id, &path)?;

            transaction.commit().await?;

            Ok(())
        })
    }

    fn create_empty_file(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<()> {
        let database = self.database.clone();
        let id = id.clone();
        let path = path.clone();

        AsyncJob::new(async move {
            if Self::entry_exists(&database, &id, &path).await? {
                return Err(AppError::PathAlreadyExists(path));
            }

            let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
            let files_store = Self::files_store(&transaction)?;
            let directories_store = Self::directories_store(&transaction)?;

            Self::ensure_parent_directories(&directories_store, &id, &path)?;

            let key = Self::key(&id, &path);
            files_store
                .add(Uint8Array::from(Vec::new()))
                .with_key(key)
                .build()?;

            transaction.commit().await?;

            Ok(())
        })
    }

    fn read(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<u8>> {
        let id = id.clone();
        let database = self.database.clone();
        let path = path.clone();

        AsyncJob::new(Self::read_bytes(database, id, path))
    }

    fn read_to_string(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<String> {
        let database = self.database.clone();
        let id = id.clone();
        let path = path.clone();

        AsyncJob::new(async move {
            let bytes = Self::read_bytes(database, id, path.clone()).await?;

            String::from_utf8(bytes).map_err(|_| AppError::FileNotValidUtf8(path))
        })
    }

    fn list_entries(&self, id: &ProjectIdentifier) -> FutureResult<FileSystemEntries> {
        let database = self.database.clone();
        let id = id.clone();

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

    fn delete_path(&self, id: &ProjectIdentifier, path: &FilePath) -> FutureResult<Vec<FilePath>> {
        let database = self.database.clone();
        let id = id.clone();
        let path = path.clone();

        AsyncJob::new(async move {
            let transaction = Self::entries_transaction(&database, TransactionMode::Readonly)?;
            let files_store = Self::files_store(&transaction)?;
            let directories_store = Self::directories_store(&transaction)?;

            let (file_entries, directory_entries) = futures_lite::future::try_zip(
                Self::keyed_paths_under_path(&id, &path, files_store),
                Self::keyed_paths_under_path(&id, &path, directories_store),
            )
            .await?;

            let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
            let files_store = Self::files_store(&transaction)?;
            let directories_store = Self::directories_store(&transaction)?;

            let mut changed_paths =
                Vec::with_capacity(file_entries.len() + directory_entries.len());
            changed_paths.extend(file_entries.iter().map(|(_, path)| path.clone()));
            changed_paths.extend(directory_entries.iter().map(|(_, path)| path.clone()));

            if changed_paths.is_empty() {
                return Err(AppError::FileNotFound(path));
            }

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

    fn move_path(
        &self,
        id: &ProjectIdentifier,
        old: &FilePath,
        new: &FilePath,
    ) -> FutureResult<Vec<FilePath>> {
        let database = self.database.clone();
        let id = id.clone();
        let old = old.clone();
        let new = new.clone();

        AsyncJob::new(async move {
            if old == new {
                return Ok(Vec::new());
            }

            if new.starts_with(&old) {
                return Err(AppError::PathAlreadyExists(new));
            }

            if Self::entry_exists(&database, &id, &new).await? {
                return Err(AppError::PathAlreadyExists(new));
            }

            match Self::read_bytes(database.clone(), id.clone(), old.clone()).await {
                Ok(bytes) => {
                    let transaction =
                        Self::entries_transaction(&database, TransactionMode::Readwrite)?;
                    let files_store = Self::files_store(&transaction)?;
                    let directories_store = Self::directories_store(&transaction)?;

                    Self::ensure_parent_directories(&directories_store, &id, &new)?;

                    files_store
                        .add(Uint8Array::from(bytes))
                        .with_key(Self::key(&id, &new))
                        .build()?;
                    files_store.delete(Self::key(&id, &old)).build()?;

                    transaction.commit().await?;

                    return Ok(vec![old, new]);
                }
                Err(AppError::FileNotFound(_)) => {}
                Err(error) => return Err(error),
            }

            if !Self::entry_exists(&database, &id, &old).await? {
                return Err(AppError::FileNotFound(old));
            }

            let transaction = Self::entries_transaction(&database, TransactionMode::Readonly)?;
            let directories_store = Self::directories_store(&transaction)?;

            let directory_entries =
                Self::keyed_paths_under_path(&id, &old, directories_store).await?;
            drop(transaction);
            let file_entries = Self::file_entries_under_path(&database, &id, &old).await?;

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
                    .replace_prefix(&old, &new)
                    .expect("directory entry was collected from the old path");

                changed_paths.push(moved_path.clone());
                moved_directories.push((key, moved_path));
            }

            let mut moved_files = Vec::with_capacity(file_entries.len());
            for (key, file_path, bytes) in file_entries {
                let moved_path = file_path
                    .replace_prefix(&old, &new)
                    .expect("file entry was collected from the old path");

                changed_paths.push(moved_path.clone());
                moved_files.push((key, moved_path, bytes));
            }

            let transaction = Self::entries_transaction(&database, TransactionMode::Readwrite)?;
            let files_store = Self::files_store(&transaction)?;
            let directories_store = Self::directories_store(&transaction)?;

            Self::ensure_parent_directories(&directories_store, &id, &new)?;

            for (_, moved_path) in &moved_directories {
                directories_store
                    .put(JsValue::TRUE)
                    .with_key(Self::key(&id, moved_path))
                    .build()?;
            }
            for (_, moved_path, bytes) in &moved_files {
                files_store
                    .add(Uint8Array::from(bytes.clone()))
                    .with_key(Self::key(&id, moved_path))
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
}
