use serde::{Deserialize, de::DeserializeOwned};

use crate::{
    error::{AppError, AppResult},
    project::paths::FilePath,
    utils::github::download::DownloadTask,
};

#[derive(Debug, Clone)]
pub struct GitRepository {
    pub user: String,
    pub repo: String,
    pub git_ref: String,
}

impl GitRepository {
    pub fn new(
        user: impl Into<String>,
        repo: impl Into<String>,
        git_ref: impl Into<String>,
    ) -> Self {
        Self {
            user: user.into(),
            repo: repo.into(),
            git_ref: git_ref.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitTreeResponse {
    pub tree: Vec<GitTreeItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitTreeItem {
    pub path: FilePath,
    #[serde(rename = "type")]
    pub item_type: String,
}

pub fn download_files_under_path(repository: &GitRepository, path: &FilePath) -> DownloadTask {
    DownloadTask::new(repository.clone(), path.clone())
}

pub async fn list_files(repository: &GitRepository) -> AppResult<Vec<GitTreeItem>> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        repository.user, repository.repo, repository.git_ref
    );

    let tree: GitTreeResponse = get_json(&url).await?;
    Ok(tree.tree)
}

pub async fn download_file(repository: &GitRepository, file_path: &FilePath) -> AppResult<Vec<u8>> {
    let file_path = file_path.to_string();
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        repository.user, repository.repo, repository.git_ref, file_path
    );

    log::info!("Downloading file {file_path} from {url}...");

    let content = get(&url).await?;
    Ok(content)
}

pub async fn get(url: &str) -> AppResult<Vec<u8>> {
    let request = ehttp::Request::get(url);
    let response = ehttp::fetch_async(request).await;
    let response = response.map_err(AppError::FetchError)?;
    Ok(response.bytes)
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> AppResult<T> {
    let content = get(url).await?;
    Ok(serde_json::from_slice(&content)?)
}

pub mod download {
    use std::task::Poll;

    use crate::utils::async_job::AsyncJob;

    use super::*;

    #[derive(Debug)]
    pub enum DownloadTask {
        Listing {
            repository: GitRepository,
            path: FilePath,
            task: AsyncJob<AppResult<Vec<GitTreeItem>>>,
        },
        Downloading {
            repository: GitRepository,
            pending_files: Vec<(FilePath, FilePath)>,
            downloaded_files: Vec<(FilePath, Vec<u8>)>,
            current_download: Option<(FilePath, AsyncJob<AppResult<Vec<u8>>>)>,
        },
        Done {
            files: Vec<(FilePath, Vec<u8>)>,
        },
        Errored {
            error: AppError,
        },
    }

    impl DownloadTask {
        pub fn new(repository: GitRepository, path: FilePath) -> Self {
            let list_repository = repository.clone();
            let task = AsyncJob::new(async move { list_files(&list_repository).await });

            Self::Listing {
                repository,
                path,
                task,
            }
        }

        pub fn file_count_progress(&self) -> Option<(usize, usize)> {
            match self {
                DownloadTask::Downloading {
                    pending_files,
                    downloaded_files,
                    current_download,
                    ..
                } => {
                    let downloaded = downloaded_files.len();
                    let pending = pending_files.len();

                    let total = downloaded + pending + current_download.is_some() as usize;

                    Some((downloaded, total))
                }
                DownloadTask::Done { files, .. } => Some((files.len(), files.len())),
                DownloadTask::Listing { .. } | DownloadTask::Errored { .. } => None,
            }
        }

        pub fn tick(&mut self) {
            match self {
                DownloadTask::Listing {
                    repository,
                    path,
                    task,
                } => match task.try_resolve() {
                    Poll::Pending => {}
                    Poll::Ready(Ok(tree)) => {
                        let pending_files = pending_downloads_under_path(path, tree);

                        let contains_project_json =
                            pending_files.iter().any(|(path, _)| path.is_project_json());

                        if !contains_project_json {
                            *self = DownloadTask::Errored {
                                error: AppError::MissingProjectJson,
                            };
                            return;
                        }

                        *self = DownloadTask::Downloading {
                            repository: repository.clone(),
                            pending_files,
                            downloaded_files: Vec::new(),
                            current_download: None,
                        };
                        self.tick(); // Continue progress
                    }
                    Poll::Ready(Err(error)) => *self = DownloadTask::Errored { error },
                },
                DownloadTask::Downloading {
                    repository,
                    pending_files,
                    downloaded_files,
                    current_download,
                } => {
                    if let Some((relative_path, task)) = current_download {
                        match task.try_resolve() {
                            Poll::Pending => {}
                            Poll::Ready(Ok(content)) => {
                                let relative_path = relative_path.clone();
                                downloaded_files.push((relative_path, content));
                                *current_download = None;

                                if pending_files.is_empty() {
                                    let files = std::mem::take(downloaded_files);
                                    *self = DownloadTask::Done { files };
                                } else {
                                    self.tick(); // Continue progress
                                }
                            }
                            Poll::Ready(Err(error)) => *self = DownloadTask::Errored { error },
                        }
                    } else {
                        if let Some((source_path, relative_path)) = pending_files.pop() {
                            let repository = repository.clone();
                            let task_path = source_path.clone();

                            let task = async move { download_file(&repository, &task_path).await };
                            let task = AsyncJob::new(task);

                            *current_download = Some((relative_path, task));
                            self.tick(); // Continue progress
                        } else {
                            let files = std::mem::take(downloaded_files);
                            *self = DownloadTask::Done { files };
                        }
                    }
                }
                DownloadTask::Done { .. } | DownloadTask::Errored { .. } => {}
            };
        }
    }

    fn pending_downloads_under_path(
        path: &FilePath,
        tree: Vec<GitTreeItem>,
    ) -> Vec<(FilePath, FilePath)> {
        tree.into_iter()
            .filter(|tree_item| tree_item.item_type == "blob" && tree_item.path.starts_with(path))
            .map(|tree_item| {
                let relative_path = tree_item
                    .path
                    .strip_prefix(path)
                    .unwrap_or_else(|| tree_item.path.clone());
                (tree_item.path, relative_path)
            })
            .collect()
    }
}
