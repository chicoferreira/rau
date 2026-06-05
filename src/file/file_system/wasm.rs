use std::{collections::BTreeSet, sync::mpsc::Sender};

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
    file::{
        file_system::{
            AppFileSystemTrait, FileSystemEntries, FutureResult, ProjectFileSystemTrait,
        },
        file_watcher::{FileWatcher, send_all},
        identifier::ProjectIdentifier,
    },
    project::paths::FilePath,
    utils::async_job::AsyncJob,
};

const DB_NAME: &str = "rau";
const FILES_STORE: &str = "files";
const DIRECTORIES_STORE: &str = "directories";

#[derive(Clone)]
pub struct AppFileSystem {
    database: Database,
}

#[derive(Clone)]
pub struct ProjectFileSystem {
    id: ProjectIdentifier,
    database: Database,
    /// Feeds the manual file watcher: IndexedDB has no change events, so each
    /// write reports its own changed paths.
    change_sender: Sender<FilePath>,
}

impl AppFileSystemTrait for AppFileSystem {
    fn open() -> FutureResult<Self> {
        AsyncJob::new(async move {
            Ok(Self {
                database: ProjectFileSystem::open_database().await?,
            })
        })
    }

    fn mount_project(
        &self,
        id: ProjectIdentifier,
    ) -> FutureResult<(super::ProjectFileSystem, FileWatcher)> {
        let database = self.database.clone();

        AsyncJob::new(async move {
            let (change_sender, file_watcher) = FileWatcher::manual();

            let file_system = ProjectFileSystem {
                database,
                id,
                change_sender,
            };
            let file_system = super::ProjectFileSystem::IndexedDb(file_system);
            Ok((file_system, file_watcher))
        })
    }

    fn recent_projects(&self) -> FutureResult<Vec<ProjectIdentifier>> {
        let database = self.database.clone();

        AsyncJob::new(async move {
            let transaction =
                ProjectFileSystem::entries_transaction(&database, TransactionMode::Readonly)?;
            let files_store = ProjectFileSystem::files_store(&transaction)?;
            let directories_store = ProjectFileSystem::directories_store(&transaction)?;

            let (file_project_names, directory_project_names) = futures_lite::future::try_zip(
                ProjectFileSystem::project_names_in_store(files_store),
                ProjectFileSystem::project_names_in_store(directories_store),
            )
            .await?;

            let mut project_names = file_project_names;
            project_names.extend(directory_project_names);

            Ok(project_names
                .into_iter()
                .map(ProjectIdentifier::new)
                .collect())
        })
    }

    fn remember_project(&self, _id: ProjectIdentifier) -> FutureResult<()> {
        // The project will be already remembered in the database by its key whenever a file is saved.
        AsyncJob::new(async move { Ok(()) })
    }

    fn ensure_project_can_be_created(&self, id: ProjectIdentifier) -> FutureResult<()> {
        let database = self.database.clone();

        AsyncJob::new(async move {
            let transaction =
                ProjectFileSystem::entries_transaction(&database, TransactionMode::Readonly)?;
            let files_store = ProjectFileSystem::files_store(&transaction)?;
            let directories_store = ProjectFileSystem::directories_store(&transaction)?;

            let (file_keys, directory_keys) = futures_lite::future::try_zip(
                ProjectFileSystem::project_keys_in_store(&id, files_store),
                ProjectFileSystem::project_keys_in_store(&id, directories_store),
            )
            .await?;

            if !file_keys.is_empty() || !directory_keys.is_empty() {
                return Err(AppError::ProjectNameAlreadyExists(
                    id.project_name().to_string(),
                ));
            }

            Ok(())
        })
    }

    fn remove_recent_project(&self, id: ProjectIdentifier) -> FutureResult<()> {
        // The wasm platform should actually delete the project instead of just removing it from a list.
        let database = self.database.clone();

        AsyncJob::new(async move {
            let transaction =
                ProjectFileSystem::entries_transaction(&database, TransactionMode::Readonly)?;
            let files_store = ProjectFileSystem::files_store(&transaction)?;
            let directories_store = ProjectFileSystem::directories_store(&transaction)?;

            let (file_keys, directory_keys) = futures_lite::future::try_zip(
                ProjectFileSystem::project_keys_in_store(&id, files_store),
                ProjectFileSystem::project_keys_in_store(&id, directories_store),
            )
            .await?;
            drop(transaction);

            let transaction =
                ProjectFileSystem::entries_transaction(&database, TransactionMode::Readwrite)?;
            let files_store = ProjectFileSystem::files_store(&transaction)?;
            let directories_store = ProjectFileSystem::directories_store(&transaction)?;

            for key in file_keys {
                files_store.delete(key).build()?;
            }
            for key in directory_keys {
                directories_store.delete(key).build()?;
            }

            transaction.commit().await?;

            Ok(())
        })
    }
}

impl ProjectFileSystem {
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

        FilePath::new(segments).ok()
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

    async fn project_names_in_store(store: ObjectStore<'_>) -> AppResult<BTreeSet<String>> {
        Ok(store
            .get_all_keys()
            .primitive()?
            .await?
            .collect::<indexed_db_futures::Result<Vec<Array>>>()?
            .into_iter()
            .filter_map(|key| key.get(0).as_string())
            .collect())
    }

    async fn project_keys_in_store(
        identifier: &ProjectIdentifier,
        store: ObjectStore<'_>,
    ) -> AppResult<Vec<Array>> {
        Ok(store
            .get_all_keys()
            .primitive()?
            .await?
            .collect::<indexed_db_futures::Result<Vec<Array>>>()?
            .into_iter()
            .filter(|key| {
                key.get(0)
                    .as_string()
                    .is_some_and(|project_name| project_name == identifier.project_name())
            })
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

impl ProjectFileSystemTrait for ProjectFileSystem {
    fn write(&self, path: &FilePath, bytes: Vec<u8>) -> FutureResult<()> {
        let database = self.database.clone();
        let id = self.id.clone();
        let change_sender = self.change_sender.clone();
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

            let _ = change_sender.send(path);

            Ok(())
        })
    }

    fn create_directory(&self, path: &FilePath) -> FutureResult<()> {
        let database = self.database.clone();
        let id = self.id.clone();
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

    fn read(&self, path: &FilePath) -> FutureResult<Vec<u8>> {
        let id = self.id.clone();
        let database = self.database.clone();
        let path = path.clone();

        AsyncJob::new(Self::read_bytes(database, id, path))
    }

    fn exists(&self, path: &FilePath) -> FutureResult<bool> {
        let database = self.database.clone();
        let id = self.id.clone();
        let path = path.clone();

        AsyncJob::new(async move { Self::entry_exists(&database, &id, &path).await })
    }

    fn list_entries(&self) -> FutureResult<FileSystemEntries> {
        let database = self.database.clone();
        let id = self.id.clone();

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

    fn delete_path(&self, path: &FilePath) -> FutureResult<()> {
        let database = self.database.clone();
        let id = self.id.clone();
        let change_sender = self.change_sender.clone();
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

            if file_entries.is_empty() && directory_entries.is_empty() {
                return Err(AppError::FileNotFound(path));
            }

            for (key, changed_path) in file_entries {
                files_store.delete(key).build()?;
                let _ = change_sender.send(changed_path);
            }
            for (key, changed_path) in directory_entries {
                directories_store.delete(key).build()?;
                let _ = change_sender.send(changed_path);
            }

            transaction.commit().await?;

            Ok(())
        })
    }

    fn move_path(&self, old: &FilePath, new: &FilePath) -> FutureResult<()> {
        let database = self.database.clone();
        let id = self.id.clone();
        let change_sender = self.change_sender.clone();
        let old = old.clone();
        let new = new.clone();

        AsyncJob::new(async move {
            if old == new {
                return Ok(());
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

                    send_all(&change_sender, [old, new]);

                    return Ok(());
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

            let mut moved_directories = Vec::with_capacity(directory_entries.len());
            for (key, directory_path) in directory_entries {
                let moved_path = directory_path
                    .replace_prefix(&old, &new)
                    .expect("directory entry was collected from the old path");

                let _ = change_sender.send(directory_path);
                let _ = change_sender.send(moved_path.clone());
                moved_directories.push((key, moved_path));
            }

            let mut moved_files = Vec::with_capacity(file_entries.len());
            for (key, file_path, bytes) in file_entries {
                let moved_path = file_path
                    .replace_prefix(&old, &new)
                    .expect("file entry was collected from the old path");

                let _ = change_sender.send(file_path);
                let _ = change_sender.send(moved_path.clone());
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

            Ok(())
        })
    }
}
