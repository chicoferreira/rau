use std::task::Poll;

use crate::{
    app::{AppEvent, State},
    error::AppResult,
    featured_projects::FEATURED_PROJECTS,
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
}

impl MainMenu {
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        for featured_project in FEATURED_PROJECTS {
            if ui.button(featured_project.name).clicked() {
                self.create_project_modal = Some(CreateProjectModal::new(
                    ProjectCreationSource::Featured(featured_project),
                ));
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

    pub fn render(&mut self, app_event_queue: &mut EventQueue<AppEvent>) {
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
    }
}
