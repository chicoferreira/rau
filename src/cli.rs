use std::path::PathBuf;

use crate::{
    StartupAction,
    error::AppResult,
    file::identifier::{ProjectIdentifier, ProjectSource},
    ui::components::create_project_modal::{GithubProjectSource, ProjectCreationSource},
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Open an existing project from a folder.
    Open { project_folder: PathBuf },
    /// Create a new project, optionally from a GitHub repository.
    New {
        #[command(subcommand)]
        storage: StorageCommand,
    },
}

/// Where the new project is stored.
#[derive(Subcommand)]
enum StorageCommand {
    /// Persistent project stored on disk in the given folder.
    Persistent {
        project_folder: PathBuf,
        #[command(subcommand)]
        source: Option<SourceCommand>,
    },
    /// Temporary in-memory project that is not saved to disk.
    Ephemeral {
        /// Project name. Defaults to the repository name, or "Untitled Project".
        #[arg(long)]
        name: Option<String>,
        #[command(subcommand)]
        source: Option<SourceCommand>,
    },
}

/// What the new project is created from.
#[derive(Subcommand)]
enum SourceCommand {
    /// An empty project (default).
    Empty,
    /// A project downloaded from a GitHub repository.
    Github {
        /// GitHub repository owner.
        #[arg(long)]
        owner: String,
        /// GitHub repository name.
        #[arg(long)]
        repo: String,
        /// Branch name or commit SHA.
        #[arg(long = "ref")]
        git_ref: String,
        /// Folder within the repository to use as the project root.
        #[arg(long)]
        path: Option<String>,
    },
}

impl Cli {
    fn into_startup_action(self) -> AppResult<StartupAction> {
        let Some(command) = self.command else {
            return Ok(StartupAction::MainMenu);
        };

        match command {
            Command::Open { project_folder } => {
                let project_id = ProjectIdentifier::extract_identifier(project_folder)?;
                Ok(StartupAction::OpenProject { project_id })
            }
            Command::New { storage } => storage.into_startup_action(),
        }
    }
}

impl StorageCommand {
    fn into_startup_action(self) -> AppResult<StartupAction> {
        match self {
            StorageCommand::Persistent {
                project_folder,
                source,
            } => {
                let creation = source.unwrap_or(SourceCommand::Empty).into_creation();
                let project_id = ProjectIdentifier::extract_identifier(project_folder)?;
                Ok(StartupAction::CreateProject {
                    source: ProjectSource::Persistent(project_id),
                    creation,
                })
            }
            StorageCommand::Ephemeral { name, source } => {
                let creation = source.unwrap_or(SourceCommand::Empty).into_creation();
                let project_name = name
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

impl SourceCommand {
    fn into_creation(self) -> ProjectCreationSource {
        match self {
            SourceCommand::Empty => ProjectCreationSource::Empty,
            SourceCommand::Github {
                owner,
                repo,
                git_ref,
                path,
            } => ProjectCreationSource::Github(GithubProjectSource {
                owner,
                repo,
                git_ref,
                path: path.unwrap_or_default(),
            }),
        }
    }
}

pub fn main() {
    env_logger::builder()
        .parse_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let startup_action = match Cli::parse().into_startup_action() {
        Ok(action) => action,
        Err(e) => {
            log::error!("Failed to parse command: {}", e);
            return;
        }
    };

    if let Err(e) = crate::run(startup_action) {
        log::error!("Failed to run: {}", e);
    }
}
