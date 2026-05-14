use std::task::Poll;

use crate::{
    app::AppEvent, app::State, error::AppResult, utils::async_job::AsyncJob,
    utils::event_queue::EventQueue, workspace::Workspace,
};

#[derive(Default)]
pub struct MainMenu {
    workspace_job: Option<AsyncJob<AppResult<Workspace>>>,
}

impl MainMenu {
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(_) = &mut self.workspace_job {
            ui.spinner();
        } else if ui.button("Open Project").clicked() {
            let workspace_job = AsyncJob::new(Workspace::new());
            self.workspace_job = Some(workspace_job);
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
