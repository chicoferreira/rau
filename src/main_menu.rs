use std::task::Poll;

use crate::{
    app::{AppEvent, State},
    error::AppResult,
    featured_projects::FEATURED_PROJECTS,
    file::app_config::{AppConfig, RecentProject},
    ui::components::create_project_modal::{
        CreateProjectModal, CreateProjectModalResponse, ProjectCreationSource,
    },
    utils::{async_job::AsyncJob, event_queue::EventQueue},
    workspace::Workspace,
};

#[derive(Default)]
pub struct MainMenu {
    workspace_job: Option<AsyncJob<AppResult<Workspace>>>,
    create_project_modal: Option<CreateProjectModal>,
    pending_remove: Option<PendingRemove>,
}

struct PendingRemove {
    index: usize,
    task: AsyncJob<AppResult<()>>,
}

impl MainMenu {
    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        recent_projects: &[RecentProject],
        app_config: &AppConfig,
    ) {
        ui.heading("Recent Projects");

        if recent_projects.is_empty() {
            ui.label("No recent projects");
        }

        for (index, project) in recent_projects.iter().enumerate() {
            let is_removing = self
                .pending_remove
                .as_ref()
                .is_some_and(|r| r.index == index);

            ui.horizontal(|ui| {
                ui.add_enabled_ui(!is_removing, |ui| {
                    if ui.button(&project.project_name).clicked() {
                        let id = project.to_project_identifier();
                        let workspace_job = Workspace::open_existing(id);
                        self.workspace_job = Some(AsyncJob::new(workspace_job));
                    }

                    let remove_label = remove_button_label();

                    if ui.button(remove_label).clicked() && self.pending_remove.is_none() {
                        let task = app_config.remove_recent(project);
                        self.pending_remove = Some(PendingRemove { index, task });
                    }
                });

                if is_removing {
                    ui.spinner();
                }
            });
        }

        ui.separator();

        ui.heading("Featured Projects");

        for featured_project in FEATURED_PROJECTS {
            if ui.button(featured_project.name).clicked() {
                self.create_project_modal =
                    Some(CreateProjectModal::from_featured_project(featured_project));
            }
        }

        if ui.button("New Project").clicked() {
            self.create_project_modal = Some(CreateProjectModal::new(ProjectCreationSource::Empty));
        }

        if let Some(modal) = &mut self.create_project_modal {
            if let Some(response) = modal.render_ui(ui) {
                match response {
                    CreateProjectModalResponse::Create {
                        project_identifier: project_id,
                        files,
                    } => {
                        let workspace_job = Workspace::new_project_from_files(project_id, files);
                        let workspace_job = AsyncJob::new(workspace_job);
                        self.workspace_job = Some(workspace_job);
                        self.create_project_modal = None;
                    }
                    CreateProjectModalResponse::Close => {
                        self.create_project_modal = None;
                    }
                }
            }
        }
    }

    pub fn render(
        &mut self,
        app_event_queue: &mut EventQueue<AppEvent>,
        recent_projects: &mut Vec<RecentProject>,
    ) {
        if let Some(job) = &mut self.workspace_job {
            if let Poll::Ready(result) = job.try_resolve() {
                match result {
                    Ok(workspace) => {
                        app_event_queue.add(AppEvent::SetState(State::Workspace(workspace)));
                    }
                    Err(error) => {
                        log::error!("Failed to open workspace: {error:?}");
                    }
                }
                self.workspace_job = None;
            }
        }

        if let Some(pending) = &mut self.pending_remove {
            if let Poll::Ready(result) = pending.task.try_resolve() {
                match result {
                    Ok(()) => {
                        if pending.index < recent_projects.len() {
                            recent_projects.remove(pending.index);
                        }
                    }
                    Err(error) => {
                        log::error!("Failed to remove recent project: {error:?}");
                    }
                }
                self.pending_remove = None;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn remove_button_label() -> &'static str {
    "Remove from recents"
}

#[cfg(target_arch = "wasm32")]
fn remove_button_label() -> &'static str {
    "Delete"
}
