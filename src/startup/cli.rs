use std::path::PathBuf;

use crate::{
    StartupAction, StartupSettings,
    app::AppSettings,
    error::AppResult,
    file::identifier::{ProjectIdentifier, ProjectSource},
    scene::{self, GenerateTemplate},
    ui::components::create_project_modal::{GithubProjectSource, ProjectCreationSource},
    utils::{benchmark::BenchmarkSettings, render_settings::RenderSettings},
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    render_settings: RenderSettings,

    #[command(flatten)]
    window_settings: WindowSettings,

    #[command(flatten)]
    benchmark: BenchmarkSettings,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Settings the window is created with.
#[derive(clap::Args)]
pub struct WindowSettings {
    /// Window width at startup.
    #[arg(long = "window-width", default_value_t = 1280, global = true)]
    pub width: u32,

    /// Window height at startup.
    #[arg(long = "window-height", default_value_t = 800, global = true)]
    pub height: u32,
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
    /// Generate a bundled example project into a target folder.
    Generate {
        /// Which example project to generate.
        template: GenerateTemplate,
        /// Folder to write the generated project into.
        target_folder: PathBuf,
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

    let cli = Cli::parse();

    let action = match cli.command {
        None => Ok(StartupAction::MainMenu),
        Some(Command::Open { project_folder }) => {
            ProjectIdentifier::extract_identifier(project_folder)
                .map(|project_id| StartupAction::OpenProject { project_id })
        }
        Some(Command::New { storage }) => storage.into_startup_action(),
        Some(Command::Generate {
            template,
            target_folder,
        }) => {
            if let Err(e) = scene::generate_project(template, &target_folder) {
                log::error!("Failed to generate project: {}", e);
            }
            return;
        }
    };

    let startup_action = match action {
        Ok(action) => action,
        Err(e) => {
            log::error!("Failed to parse command: {}", e);
            return;
        }
    };

    let settings = StartupSettings {
        app: AppSettings {
            action: startup_action,
            benchmark: cli.benchmark,
        },
        render_settings: cli.render_settings,
        window_settings: cli.window_settings,
    };

    if let Err(e) = crate::run(settings) {
        log::error!("Failed to run: {}", e);
    }
}
