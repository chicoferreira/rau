use std::task::Poll;

use crate::{
    app::{AppEvent, State},
    error::AppResult,
    featured_projects::FEATURED_PROJECTS,
    file::{file_system::AppFileSystem, identifier::ProjectIdentifier},
    project::paths::FilePath,
    ui::components::{
        create_project_modal::{
            CreateProjectModal, CreateProjectModalResponse, ProjectCreationSource,
        },
        recent_projects::RecentProjectsState,
    },
    utils::{async_job::AsyncJob, event_queue::EventQueue},
    workspace::Workspace,
};

#[derive(Default)]
pub struct MainMenu {
    workspace_job: Option<AsyncJob<AppResult<Workspace>>>,
    recent_projects_state: RecentProjectsState,
    create_project_modal: Option<CreateProjectModal>,
}

impl MainMenu {
    pub fn render_ui(&mut self, ui: &mut egui::Ui, app_fs: &AppFileSystem) {
        for featured_project in FEATURED_PROJECTS {
            if ui.button(featured_project.name).clicked() {
                self.create_project_modal =
                    Some(CreateProjectModal::from_featured_project(featured_project));
            }
        }

        if ui.button("New Project").clicked() {
            self.create_project_modal = Some(CreateProjectModal::new(ProjectCreationSource::Empty));
        }

        let open_pending = self.workspace_job.is_some();
        if let Some(project_id) = self
            .recent_projects_state
            .render_ui(ui, open_pending, app_fs)
        {
            self.open_project(app_fs.clone(), project_id, vec![]);
        }

        if let Some(modal) = &mut self.create_project_modal {
            if let Some(response) = modal.render_ui(ui) {
                match response {
                    CreateProjectModalResponse::Create {
                        project_identifier: project_id,
                        files,
                    } => {
                        self.open_project(app_fs.clone(), project_id, files);
                        self.create_project_modal = None;
                    }
                    CreateProjectModalResponse::Close => {
                        self.create_project_modal = None;
                    }
                }
            }
        }
    }

    fn open_project(
        &mut self,
        app_fs: AppFileSystem,
        project_id: ProjectIdentifier,
        files: Vec<(FilePath, Vec<u8>)>,
    ) {
        let workspace_job = Workspace::open_project_and_save_files(app_fs, project_id, files);
        let workspace_job = AsyncJob::new(workspace_job);
        self.workspace_job = Some(AsyncJob::new(workspace_job));
    }

    pub fn render(
        &mut self,
        app_event_queue: &mut EventQueue<AppEvent>,
        app_file_system: &AppFileSystem,
    ) {
        self.recent_projects_state.tick(app_file_system);

        if let Some(job) = &mut self.workspace_job {
            if let Poll::Ready(result) = job.try_resolve() {
                match result {
                    Ok(workspace) => {
                        app_event_queue.add(AppEvent::SetState(State::Workspace(workspace)));
                    }
                    Err(error) => {
                        log::error!("Failed to open workspace: {error:?}");
                        self.recent_projects_state.reload();
                    }
                }
                self.workspace_job = None;
            }
        }
    }
}
