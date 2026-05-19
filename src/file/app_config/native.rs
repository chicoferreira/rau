use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    file::{absolute::AbsolutePathBuf, app_config::RecentProject, identifier::ProjectIdentifier},
    utils::async_job::AsyncJob,
};

#[derive(Serialize, Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    recent_projects: Vec<RecentProjectEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RecentProjectEntry {
    name: String,
    path: String,
}

pub struct AppConfig {
    config_path: PathBuf,
}

impl AppConfig {
    pub fn load() -> AsyncJob<AppResult<(Self, Vec<RecentProject>)>> {
        AsyncJob::new(async {
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("rau");

            std::fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("config.toml");

            let config = if config_path.exists() {
                let contents = std::fs::read_to_string(&config_path)?;
                toml::from_str::<ConfigFile>(&contents).unwrap_or_default()
            } else {
                ConfigFile::default()
            };

            let recent = config
                .recent_projects
                .into_iter()
                .filter_map(|entry| {
                    let project_path = AbsolutePathBuf::try_from(entry.path.as_str()).ok()?;
                    Some(RecentProject {
                        project_name: entry.name,
                        project_path,
                    })
                })
                .collect();

            Ok((Self { config_path }, recent))
        })
    }

    pub fn add_recent(&self, identifier: &ProjectIdentifier) -> AsyncJob<AppResult<()>> {
        let config_path = self.config_path.clone();
        let name = identifier.project_name().to_string();
        let path = identifier.project_path().as_ref().display().to_string();

        AsyncJob::new(async move {
            let mut config = read_config(&config_path);

            config
                .recent_projects
                .retain(|entry| entry.name != name || entry.path != path);

            config
                .recent_projects
                .insert(0, RecentProjectEntry { name, path });

            write_config(&config_path, &config)?;
            Ok(())
        })
    }

    pub fn remove_recent(&self, project: &RecentProject) -> AsyncJob<AppResult<()>> {
        let config_path = self.config_path.clone();
        let name = project.project_name.clone();
        let path = project.project_path.as_ref().display().to_string();

        AsyncJob::new(async move {
            let mut config = read_config(&config_path);

            config
                .recent_projects
                .retain(|entry| entry.name != name || entry.path != path);

            write_config(&config_path, &config)?;
            Ok(())
        })
    }
}

fn read_config(config_path: &PathBuf) -> ConfigFile {
    if config_path.exists() {
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|contents| toml::from_str(&contents).ok())
            .unwrap_or_default()
    } else {
        ConfigFile::default()
    }
}

fn write_config(config_path: &PathBuf, config: &ConfigFile) -> AppResult<()> {
    let contents =
        toml::to_string_pretty(config).expect("ConfigFile serialization should not fail");
    std::fs::write(config_path, contents)?;
    Ok(())
}
