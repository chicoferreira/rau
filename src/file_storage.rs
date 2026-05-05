use std::task::Poll;

use crate::{
    error::AppResult,
    fs::{file_system::FileSystem, file_watcher::FileWatcher, identifier::ProjectIdentifier},
    project::{paths::FilePath, sync::SyncTracker},
    utils::{async_job::AsyncJob, dir_node::DirNode},
};

/// A struct that holds files of the project for the UI
/// to display without having to poll the file system.
pub struct FileStorage {
    pub file_system: FileSystem,
    project_id: ProjectIdentifier,
    file_watcher: FileWatcher,
    current_tasks: Vec<FileStorageTask>,
    cached_files: Option<Vec<FilePath>>,
    cached_file_tree: Option<DirNode>,
}

enum FileStorageTask {
    ListFiles {
        task: AsyncJob<AppResult<Vec<FilePath>>>,
    },
    CreateFile {
        task: AsyncJob<AppResult<()>>,
    },
    DeleteFile {
        path: FilePath,
        task: AsyncJob<AppResult<()>>,
    },
}

impl FileStorage {
    pub async fn new(project_identifier: ProjectIdentifier) -> AppResult<Self> {
        let file_system = FileSystem::new().await?;
        let file_watcher = file_system.create_file_watcher(&project_identifier)?;
        Ok(Self {
            file_system,
            project_id: project_identifier,
            cached_files: None,
            cached_file_tree: None,
            current_tasks: vec![],
            file_watcher,
        })
    }

    pub fn project_identifier(&self) -> &ProjectIdentifier {
        &self.project_id
    }

    pub fn file_tree(&mut self) -> Option<&DirNode> {
        if self.cached_file_tree.is_none() && !self.has_list_file_files_pending() {
            self.refresh_file_system();
        }
        self.cached_file_tree.as_ref()
    }

    fn has_list_file_files_pending(&self) -> bool {
        self.current_tasks
            .iter()
            .any(|task| matches!(task, FileStorageTask::ListFiles { .. }))
    }

    fn refresh_file_system(&mut self) {
        self.current_tasks.push(FileStorageTask::ListFiles {
            task: self.file_system.list_files(&self.project_id),
        });
    }

    pub fn exists_file_cached(&self, path: &FilePath) -> bool {
        self.cached_files
            .as_ref()
            .map_or(false, |files| files.iter().any(|f| f == path))
    }

    pub fn read(&self, path: &FilePath) -> AsyncJob<AppResult<Vec<u8>>> {
        self.file_system.read(&self.project_id, path)
    }

    pub fn read_to_string(&self, path: &FilePath) -> AsyncJob<AppResult<String>> {
        self.file_system.read_to_string(&self.project_id, path)
    }

    pub fn create_file_in_background(&mut self, parent_path: FilePath, new_name: String) {
        let file_path = parent_path.join(new_name);

        let task = self
            .file_system
            .create_empty_file(&self.project_id, &file_path);

        let task = FileStorageTask::CreateFile { task };
        self.current_tasks.push(task);
    }

    pub fn rename_file_in_background(&self, _file_path: FilePath, _new_name: String) {
        todo!()
    }

    pub fn tick(&mut self, tracker: &mut SyncTracker) {
        // Handle file watcher events
        while let Some(result) = self.file_watcher.try_next() {
            match result {
                Ok(paths) => {
                    tracker.push_file_changes(paths);
                    self.refresh_file_system();
                }
                Err(e) => log::error!("File watcher error: {}", e),
            }
        }

        let mut refresh_file_system = false;

        self.current_tasks.retain_mut(|task| match task {
            FileStorageTask::ListFiles { task } => {
                consume_if_ready(task, "list files", |mut files| {
                    files.sort_by_key(|file| file.segments().to_vec());
                    self.cached_file_tree = Some(DirNode::from_files(&files));
                    self.cached_files = Some(files);
                })
            }
            FileStorageTask::CreateFile { task } => {
                consume_if_ready(task, "create file", |_| refresh_file_system = true)
            }
            FileStorageTask::DeleteFile { task, path } => {
                consume_if_ready(task, "delete file", |_| {
                    refresh_file_system = true;
                    tracker.push_file_change(path.clone());
                })
            }
        });

        if refresh_file_system && !self.has_list_file_files_pending() {
            self.refresh_file_system();
        }
    }

    pub fn delete_file_in_background(&mut self, file_path: FilePath) {
        self.current_tasks.push(FileStorageTask::DeleteFile {
            task: self.file_system.delete_file(&self.project_id, &file_path),
            path: file_path,
        });
    }
}

fn consume_if_ready<T>(job: &mut AsyncJob<AppResult<T>>, name: &str, f: impl FnOnce(T)) -> bool {
    match job.try_resolve() {
        Poll::Ready(result) => {
            match result {
                Ok(value) => f(value),
                Err(e) => log::error!("Failed to {name}: {}", e),
            }
            false
        }
        Poll::Pending => true,
    }
}
