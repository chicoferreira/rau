use serde::Deserialize;

use crate::project::paths::FilePath;

pub struct GitRepository {
    pub user: String,
    pub repo: String,
    pub branch: String,
}

impl GitRepository {
    pub fn new(
        user: impl Into<String>,
        repo: impl Into<String>,
        branch: impl Into<String>,
    ) -> Self {
        Self {
            user: user.into(),
            repo: repo.into(),
            branch: branch.into(),
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

pub async fn download_files_under_path(
    repository: &GitRepository,
    path: &FilePath,
) -> Result<Vec<(FilePath, Vec<u8>)>, reqwest::Error> {
    let mut result = Vec::new();
    for tree_item in list_files(repository).await? {
        if tree_item.item_type == "blob" {
            if tree_item.path.starts_with(path) {
                let content = download_file(repository, &tree_item.path).await?;
                let relative_path = tree_item.path.strip_prefix(path).unwrap_or(tree_item.path);
                result.push((relative_path.clone(), content));
            }
        }
    }
    Ok(result)
}

pub async fn list_files(repository: &GitRepository) -> Result<Vec<GitTreeItem>, reqwest::Error> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        repository.user, repository.repo, repository.branch
    );

    let response = reqwest::get(&url).await?;
    let tree: GitTreeResponse = response.json().await?;

    Ok(tree.tree)
}

pub async fn download_file(
    repository: &GitRepository,
    file_path: &FilePath,
) -> Result<Vec<u8>, reqwest::Error> {
    let file_path = file_path.to_string();
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/{}",
        repository.user, repository.repo, repository.branch, file_path
    );

    log::info!("Downloading file {file_path} from {url}...");

    let response = reqwest::get(&url).await?;
    let content = response.bytes().await?;
    Ok(content.to_vec())
}
