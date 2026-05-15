use std::task::Poll;

use crate::{
    app::{AppEvent, State},
    error::AppResult,
    ui::components::create_project_modal::CreateProjectModal,
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
        if ui.button("Open Project").clicked() {
            let workspace_job = AsyncJob::new(Workspace::open_example_project());
            self.workspace_job = Some(workspace_job);
        }

        if ui.button("New Project").clicked() {
            self.create_project_modal = Some(CreateProjectModal::default());
        }

        if let Some(modal) = &mut self.create_project_modal {
            if let Some(result) = modal.render_ui(ui) {
                let workspace_job = AsyncJob::new(Workspace::new_empty_project(result));
                self.workspace_job = Some(workspace_job);
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
