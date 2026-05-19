use std::collections::BTreeSet;

use indexed_db_futures::{
    Build, BuildPrimitive,
    database::Database,
    prelude::QuerySource,
    transaction::TransactionMode,
};
use web_sys::js_sys::Array;

use crate::{
    error::AppResult,
    file::{app_config::RecentProject, identifier::ProjectIdentifier, indexed_db},
    utils::async_job::AsyncJob,
};

pub struct AppConfig {
    database: Database,
}

impl AppConfig {
    pub fn load() -> AsyncJob<AppResult<(Self, Vec<RecentProject>)>> {
        AsyncJob::new(async {
            let database = indexed_db::open_database().await?;
            let projects = list_project_names(&database).await?;
            let recent = projects
                .into_iter()
                .map(|project_name| RecentProject { project_name })
                .collect();
            Ok((Self { database }, recent))
        })
    }

    pub fn add_recent(&self, _identifier: &ProjectIdentifier) -> AsyncJob<AppResult<()>> {
        AsyncJob::new(async { Ok(()) })
    }

    pub fn remove_recent(&self, project: &RecentProject) -> AsyncJob<AppResult<()>> {
        let database = self.database.clone();
        let project_name = project.project_name.clone();

        AsyncJob::new(async move {
            delete_project_entries(&database, &project_name).await?;
            Ok(())
        })
    }
}

async fn list_project_names(database: &Database) -> AppResult<Vec<String>> {
    let transaction = database
        .transaction(indexed_db::FILES_STORE)
        .with_mode(TransactionMode::Readonly)
        .build()?;
    let store = transaction.object_store(indexed_db::FILES_STORE)?;
    let keys: Vec<Array> = store
        .get_all_keys()
        .primitive()?
        .await?
        .collect::<indexed_db_futures::Result<_>>()?;

    let names: BTreeSet<String> = keys
        .into_iter()
        .filter_map(|key| key.get(0).as_string())
        .collect();

    Ok(names.into_iter().collect())
}

async fn delete_project_entries(database: &Database, project_name: &str) -> AppResult<()> {
    let transaction = database
        .transaction([indexed_db::FILES_STORE, indexed_db::DIRECTORIES_STORE])
        .with_mode(TransactionMode::Readwrite)
        .build()?;

    let files_store = transaction.object_store(indexed_db::FILES_STORE)?;
    let file_keys: Vec<Array> = files_store
        .get_all_keys()
        .primitive()?
        .await?
        .collect::<indexed_db_futures::Result<_>>()?;
    for key in file_keys {
        if key.get(0).as_string().as_deref() == Some(project_name) {
            files_store.delete(key).build()?;
        }
    }

    let directories_store = transaction.object_store(indexed_db::DIRECTORIES_STORE)?;
    let dir_keys: Vec<Array> = directories_store
        .get_all_keys()
        .primitive()?
        .await?
        .collect::<indexed_db_futures::Result<_>>()?;
    for key in dir_keys {
        if key.get(0).as_string().as_deref() == Some(project_name) {
            directories_store.delete(key).build()?;
        }
    }

    transaction.commit().await?;
    Ok(())
}
