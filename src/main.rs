use std::path::PathBuf;

use clap::{Parser, Subcommand};
use rau::{StartupAction, error::AppResult, file::identifier::ProjectIdentifier};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Open { project_folder: PathBuf },
    New { project_folder: PathBuf },
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
            Command::New { project_folder } => {
                let project_id = ProjectIdentifier::extract_identifier(project_folder)?;
                Ok(StartupAction::CreateEmptyProject { project_id })
            }
        }
    }
}

fn main() {
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

    rau::run(startup_action).unwrap();
}
