//! Web-only startup configuration read from the page URL query parameters.
//!
//! This lets the WASM build be embedded in other websites that open a project
//! directly, e.g.:
//!
//! ```text
//! ?action=new&source=github&owner=chicoferreira&repo=rau&ref=main
//! ?action=open&project=My%20Project
//! ```

use serde::Deserialize;

use crate::{
    StartupAction,
    error::{AppError, AppResult},
    file::identifier::{ProjectIdentifier, ProjectSource},
    ui::components::create_project_modal::{GithubProjectSource, ProjectCreationSource},
};

/// Open an existing project or create a new one.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Open,
    New,
}

/// Where a new project is stored.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Storage {
    Persistent,
    Ephemeral,
}

/// What a new project is created from.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Source {
    Empty,
    Github,
}

/// Flat representation of the supported URL query parameters.
#[derive(Debug, Deserialize)]
struct UrlParams {
    action: Option<Action>,
    /// Project name to open, or the name of a persistent project to create.
    project: Option<String>,
    /// Storage for a new project (defaults to `ephemeral`).
    storage: Option<Storage>,
    /// Name of a new ephemeral project.
    name: Option<String>,
    /// Source for a new project (inferred from `owner`/`repo` when omitted).
    source: Option<Source>,
    /// GitHub repository owner.
    owner: Option<String>,
    /// GitHub repository name.
    repo: Option<String>,
    /// GitHub branch name or commit SHA.
    #[serde(rename = "ref")]
    git_ref: Option<String>,
    /// Folder within the repository to use as the project root.
    path: Option<String>,
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidUrlParameters(message.into())
}

impl UrlParams {
    /// Resolves the project source a `new` project is created from.
    fn creation(&self) -> AppResult<ProjectCreationSource> {
        let is_github = match self.source {
            Some(Source::Github) => true,
            Some(Source::Empty) => false,
            // Infer GitHub when repository parameters are present.
            None => self.owner.is_some() || self.repo.is_some(),
        };

        if !is_github {
            return Ok(ProjectCreationSource::Empty);
        }

        let owner = self
            .owner
            .clone()
            .ok_or_else(|| invalid("github source requires 'owner'"))?;
        let repo = self
            .repo
            .clone()
            .ok_or_else(|| invalid("github source requires 'repo'"))?;
        let git_ref = self
            .git_ref
            .clone()
            .ok_or_else(|| invalid("github source requires 'ref'"))?;

        Ok(ProjectCreationSource::Github(GithubProjectSource {
            owner,
            repo,
            git_ref,
            path: self.path.clone().unwrap_or_default(),
        }))
    }

    fn into_startup_action(self) -> AppResult<StartupAction> {
        // Without an explicit action, leave the user on the main menu so unrelated
        // query parameters don't unexpectedly open or create a project.
        let Some(action) = self.action else {
            return Ok(StartupAction::MainMenu);
        };

        match action {
            Action::Open => {
                let project = self
                    .project
                    .ok_or_else(|| invalid("open requires 'project'"))?;
                Ok(StartupAction::OpenProject {
                    project_id: ProjectIdentifier::new(project),
                })
            }
            Action::New => {
                let creation = self.creation()?;
                match self.storage.unwrap_or(Storage::Ephemeral) {
                    Storage::Persistent => {
                        let project = self
                            .project
                            .or_else(|| creation.default_project_name())
                            .ok_or_else(|| invalid("persistent storage requires 'project'"))?;
                        Ok(StartupAction::CreateProject {
                            source: ProjectSource::Persistent(ProjectIdentifier::new(project)),
                            creation,
                        })
                    }
                    Storage::Ephemeral => {
                        let project_name = self
                            .name
                            .or_else(|| creation.default_project_name())
                            .unwrap_or_else(|| "Untitled Project".to_string());
                        Ok(StartupAction::CreateProject {
                            source: ProjectSource::Ephemeral { project_name },
                            creation,
                        })
                    }
                }
            }
        }
    }
}

/// Reads the startup action from the page URL, resetting the browser URL back to
/// the base path afterwards. Any failure falls back to [`StartupAction::MainMenu`].
pub fn startup_action_from_url() -> StartupAction {
    match try_startup_action_from_url() {
        Ok(action) => action,
        Err(e) => {
            log::error!("Failed to parse startup action from URL: {e}");
            StartupAction::MainMenu
        }
    }
}

fn try_startup_action_from_url() -> AppResult<StartupAction> {
    let Some(query) = crate::utils::browser::url::take_query_string()? else {
        return Ok(StartupAction::MainMenu);
    };

    let params: UrlParams =
        serde_urlencoded::from_str(&query).map_err(|e| invalid(e.to_string()))?;

    params.into_startup_action()
}
