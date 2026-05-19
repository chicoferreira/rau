use std::{collections::HashMap, task::Poll};

use crate::{
    error::{AppError, AppResult},
    file::{
        file_system::{FileSystemEntries, ProjectFileSystem, ProjectFileSystemTrait},
        file_watcher::FileWatcher,
        identifier::ProjectIdentifier,
    },
    project::{paths::FilePath, sync::SyncTracker},
    utils::{async_job::AsyncJob, dir_node::DirNode},
};

/// A struct that holds files of the project for the UI
/// to display without having to poll the file system.
pub struct FileStorage {
    pub file_system: ProjectFileSystem,
    project_id: ProjectIdentifier,
    file_watcher: FileWatcher,
    current_tasks: Vec<FileStorageTask>,
    cached_files: Option<Vec<FilePath>>,
    cached_file_tree: Option<DirNode>,
    open_files: HashMap<FilePath, OpenFileState>,
}

pub enum OpenFileState {
    /// The file was just opened and has no text buffer to show yet.
    Loading { task: AsyncJob<AppResult<String>> },
    /// The file is reloading from disk while keeping the previous text buffer visible.
    Reloading {
        text: String,
        saved_text: String,
        task: AsyncJob<AppResult<String>>,
    },
    /// The file has an editable text buffer and the last disk-backed text.
    Loaded { text: String, saved_text: String },
    /// The file could not be loaded as text.
    Errored { error: String },
}

enum FileStorageTask {
    ListEntries {
        task: AsyncJob<AppResult<FileSystemEntries>>,
    },
    CreateFile {
        task: AsyncJob<AppResult<()>>,
    },
    CreateDirectory {
        task: AsyncJob<AppResult<()>>,
    },
    DeletePath {
        task: AsyncJob<AppResult<()>>,
    },
    MovePath {
        task: AsyncJob<AppResult<()>>,
    },
    SaveFile {
        task: AsyncJob<AppResult<()>>,
    },
    ImportFile {
        task: AsyncJob<AppResult<bool>>,
    },
    ReplaceFile {
        task: AsyncJob<AppResult<bool>>,
    },
}

impl FileStorage {
    pub fn new(
        project_identifier: ProjectIdentifier,
        file_system: ProjectFileSystem,
        file_watcher: FileWatcher,
    ) -> Self {
        Self {
            file_system,
            project_id: project_identifier,
            cached_files: None,
            cached_file_tree: None,
            current_tasks: vec![],
            file_watcher,
            open_files: HashMap::new(),
        }
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
            .any(|task| matches!(task, FileStorageTask::ListEntries { .. }))
    }

    fn refresh_file_system(&mut self) {
        self.current_tasks.push(FileStorageTask::ListEntries {
            task: self.file_system.list_entries(),
        });
    }

    pub fn exists_file_cached(&self, path: &FilePath) -> bool {
        self.cached_files
            .as_ref()
            .map_or(false, |files| files.iter().any(|f| f == path))
    }

    pub fn read(&self, path: &FilePath) -> AsyncJob<AppResult<Vec<u8>>> {
        self.file_system.read(path)
    }

    pub fn read_to_string(&self, path: &FilePath) -> AsyncJob<AppResult<String>> {
        self.file_system.read_to_string(path)
    }

    pub fn save_in_background(&mut self, path: &FilePath, bytes: Vec<u8>) {
        let task = self.file_system.save(path, bytes);
        self.current_tasks.push(FileStorageTask::SaveFile { task });
    }

    pub fn import_file_in_background(&mut self, parent_path: FilePath) {
        let file_system = self.file_system.clone();
        let task = AsyncJob::new(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Import File")
                .pick_file()
                .await
            else {
                return Ok(false);
            };

            let file_path = parent_path.join(file.file_name())?;
            if file_system.exists(&file_path).await? {
                return Err(AppError::PathAlreadyExists(file_path));
            }

            let bytes = file.read().await;
            file_system.save(&file_path, bytes).await?;

            Ok(true)
        });

        self.current_tasks
            .push(FileStorageTask::ImportFile { task });
    }

    pub fn replace_file_in_background(&mut self, file_path: FilePath) {
        let file_system = self.file_system.clone();
        let task = AsyncJob::new(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Replace File")
                .pick_file()
                .await
            else {
                return Ok(false);
            };

            let bytes = file.read().await;
            file_system.save(&file_path, bytes).await?;

            Ok(true)
        });

        self.current_tasks
            .push(FileStorageTask::ReplaceFile { task });
    }

    pub fn get_open_file(&self, path: &FilePath) -> Option<&OpenFileState> {
        self.open_files.get(path)
    }

    pub fn open_file(&mut self, path: &FilePath) -> &mut OpenFileState {
        self.open_files
            .entry(path.clone())
            .or_insert_with(|| OpenFileState::Loading {
                task: self.file_system.read_to_string(path),
            })
    }

    pub fn create_file_in_background(&mut self, file_path: FilePath) {
        let task = self.file_system.create_empty_file(&file_path);

        let task = FileStorageTask::CreateFile { task };
        self.current_tasks.push(task);
    }

    pub fn create_folder_in_background(&mut self, folder_path: FilePath) {
        let task = self.file_system.create_directory(&folder_path);

        let task = FileStorageTask::CreateDirectory { task };
        self.current_tasks.push(task);
    }

    pub fn move_path_in_background(&mut self, old_path: FilePath, new_path: FilePath) {
        if old_path == new_path || old_path.segments().is_empty() {
            return;
        }

        let task = self.file_system.move_path(&old_path, &new_path);

        self.current_tasks.push(FileStorageTask::MovePath { task });
    }

    pub fn delete_file_in_background(&mut self, file_path: FilePath) {
        self.delete_path_in_background(file_path);
    }

    pub fn delete_folder_in_background(&mut self, path: FilePath) {
        self.delete_path_in_background(path);
    }

    fn delete_path_in_background(&mut self, path: FilePath) {
        self.current_tasks.push(FileStorageTask::DeletePath {
            task: self.file_system.delete_path(&path),
        });
    }

    pub fn tick(&mut self, tracker: &mut SyncTracker) {
        // Handle file watcher events
        while let Some(result) = self.file_watcher.try_next() {
            match result {
                Ok(paths) => {
                    self.reload_open_files(&paths);
                    tracker.push_file_changes(paths);
                    self.refresh_file_system();
                }
                Err(e) => log::error!("File watcher error: {}", e),
            }
        }

        let mut refresh_file_system = false;

        self.current_tasks.retain_mut(|task| match task {
            FileStorageTask::ListEntries { task } => {
                consume_if_ready(task, "list entries", |mut entries| {
                    entries.files.sort_by_key(|file| file.segments().to_vec());
                    entries
                        .directories
                        .sort_by_key(|directory| directory.segments().to_vec());
                    self.cached_file_tree =
                        Some(DirNode::from_entries(&entries.files, &entries.directories));
                    self.cached_files = Some(entries.files);
                })
            }
            FileStorageTask::CreateFile { task } => {
                consume_if_ready(task, "create file", |_| refresh_file_system = true)
            }
            FileStorageTask::CreateDirectory { task } => {
                consume_if_ready(task, "create directory", |_| refresh_file_system = true)
            }
            FileStorageTask::DeletePath { task } => consume_if_ready(task, "delete path", |_| {
                refresh_file_system = true;
            }),
            FileStorageTask::MovePath { task } => consume_if_ready(task, "move path", |_| {
                refresh_file_system = true;
            }),
            FileStorageTask::SaveFile { task } => consume_if_ready(task, "save file", |_| {}),
            FileStorageTask::ImportFile { task } => {
                consume_if_ready(task, "import file", |imported| {
                    if imported {
                        refresh_file_system = true;
                    }
                })
            }
            FileStorageTask::ReplaceFile { task } => {
                consume_if_ready(task, "replace file", |replaced| {
                    if replaced {
                        refresh_file_system = true;
                    }
                })
            }
        });

        if refresh_file_system && !self.has_list_file_files_pending() {
            self.refresh_file_system();
        }

        self.tick_open_files();
    }

    fn reload_open_files(&mut self, paths: &[FilePath]) {
        for path in paths {
            let Some(file) = self.open_files.get_mut(path) else {
                continue;
            };

            let task = self.file_system.read_to_string(path);
            match file {
                OpenFileState::Loaded { text, saved_text }
                | OpenFileState::Reloading {
                    text, saved_text, ..
                } => {
                    let text = text.clone();
                    let saved_text = saved_text.clone();
                    *file = OpenFileState::Reloading {
                        text,
                        saved_text,
                        task,
                    };
                }
                OpenFileState::Loading { .. } | OpenFileState::Errored { .. } => {
                    *file = OpenFileState::Loading { task };
                }
            }
        }
    }

    fn tick_open_files(&mut self) {
        for file in self.open_files.values_mut() {
            match file {
                OpenFileState::Loading { task } => {
                    let Poll::Ready(result) = task.try_resolve() else {
                        continue;
                    };

                    *file = match result {
                        Ok(text) => OpenFileState::Loaded {
                            saved_text: text.clone(),
                            text,
                        },
                        Err(error) => OpenFileState::Errored {
                            error: error.to_string(),
                        },
                    };
                }
                OpenFileState::Reloading {
                    text,
                    saved_text,
                    task,
                } => {
                    let Poll::Ready(result) = task.try_resolve() else {
                        continue;
                    };

                    *file = match result {
                        Ok(disk_text) => {
                            // A reload can finish after the user has already typed more text.
                            // If the buffer is still clean, accept the disk version into the editor.
                            if text == saved_text {
                                OpenFileState::Loaded {
                                    saved_text: disk_text.clone(),
                                    text: disk_text,
                                }
                            } else {
                                // If the buffer is dirty, keep the user's in-memory edits,
                                // while a background reload should be in progress.
                                OpenFileState::Loaded {
                                    text: text.to_string(),
                                    saved_text: disk_text,
                                }
                            }
                        }
                        Err(error) => OpenFileState::Errored {
                            error: error.to_string(),
                        },
                    };
                }
                OpenFileState::Loaded { .. } | OpenFileState::Errored { .. } => continue,
            }
        }
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
