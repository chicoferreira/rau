use std::task::Poll;

use crate::{
    error::AppResult,
    file::{
        file_system::{AppFileSystem, AppFileSystemTrait},
        identifier::ProjectIdentifier,
    },
    utils::async_job::AsyncJob,
};

#[derive(Default)]
pub struct RecentProjectsState {
    load_state: RecentProjectLoadState,
    remove_job: Option<AsyncJob<AppResult<()>>>,
}

#[derive(Default)]
pub enum RecentProjectLoadState {
    #[default]
    Pending,
    Loading {
        job: AsyncJob<AppResult<Vec<ProjectIdentifier>>>,
    },
    Loaded {
        projects: Vec<ProjectIdentifier>,
    },
}

impl RecentProjectsState {
    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        app_file_system: &AppFileSystem,
    ) -> Option<ProjectIdentifier> {
        ui.heading("Recent Projects");

        let RecentProjectLoadState::Loaded { projects } = &self.load_state else {
            ui.spinner();
            return None;
        };

        let mut result = None;

        let remove_pending = self.remove_job.is_some();

        for project in projects.clone() {
            ui.horizontal(|ui| {
                let response = ui.button(project.project_name());

                #[cfg(not(target_arch = "wasm32"))]
                let response =
                    response.on_hover_text(project.project_path().as_ref().display().to_string());

                if response.clicked() {
                    result = Some(project.clone());
                }

                ui.add_enabled_ui(!remove_pending, |ui| {
                    let remove_recent_label = cfg_select! {
                        target_arch = "wasm32" => "Delete",
                        _ => "Remove",
                    };
                    if ui.small_button(remove_recent_label).clicked() {
                        self.remove_project(app_file_system, project);
                    }
                });
            });
        }

        result
    }

    pub fn reload(&mut self) {
        self.load_state = RecentProjectLoadState::Pending;
    }

    fn remove_project(&mut self, app_file_system: &AppFileSystem, project_id: ProjectIdentifier) {
        if self.remove_job.is_some() {
            return;
        }

        self.remove_job = Some(app_file_system.remove_recent_project(project_id));
    }

    pub fn tick(&mut self, app_file_system: &AppFileSystem) {
        match &mut self.load_state {
            RecentProjectLoadState::Pending => {
                self.load_state = RecentProjectLoadState::Loading {
                    job: app_file_system.recent_projects(),
                };
            }
            RecentProjectLoadState::Loading { job } => {
                if let Poll::Ready(result) = job.try_resolve() {
                    let projects = match result {
                        Ok(projects) => projects,
                        Err(error) => {
                            log::error!("Failed to load recent projects: {error}");
                            return;
                        }
                    };
                    self.load_state = RecentProjectLoadState::Loaded { projects };
                }
            }
            RecentProjectLoadState::Loaded { .. } => {}
        }

        if let Some(job) = &mut self.remove_job {
            if let Poll::Ready(result) = job.try_resolve() {
                if let Err(error) = result {
                    log::error!("Failed to remove recent project: {error}");
                }

                self.remove_job = None;
                self.reload();
            }
        }
    }
}
