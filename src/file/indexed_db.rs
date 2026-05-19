use indexed_db_futures::{Build, database::Database};

use crate::error::AppResult;

pub const DB_NAME: &str = "rau";
pub const FILES_STORE: &str = "files";
pub const DIRECTORIES_STORE: &str = "directories";

fn has_store(database: &Database, name: &str) -> bool {
    database
        .object_store_names()
        .any(|store_name| store_name == name)
}

pub async fn open_database() -> AppResult<Database> {
    let database = Database::open(DB_NAME).await?;

    if has_store(&database, FILES_STORE) && has_store(&database, DIRECTORIES_STORE) {
        return Ok(database);
    }

    let next_version = database.version() as u32 + 1;

    database.close();

    let database = Database::open(DB_NAME)
        .with_version(next_version)
        .with_on_upgrade_needed(|_event, database| {
            if !has_store(&database, FILES_STORE) {
                database.create_object_store(FILES_STORE).build()?;
            }
            if !has_store(&database, DIRECTORIES_STORE) {
                database.create_object_store(DIRECTORIES_STORE).build()?;
            }

            Ok(())
        })
        .await?;

    Ok(database)
}
