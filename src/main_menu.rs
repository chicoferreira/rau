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
        open_or_import_project::OpenOrImportProject,
        recent_projects::RecentProjectsState,
    },
    utils::{async_job::AsyncJob, event_queue::EventQueue},
    workspace::Workspace,
};

use std::task::Poll;

#[derive(Default)]
pub struct MainMenu {
    open_workspace_job: Option<AsyncJob<AppResult<Workspace>>>,
    open_or_import_project: OpenOrImportProject,
    recent_projects_state: RecentProjectsState,
    create_project_modal: Option<CreateProjectModal>,
}

impl MainMenu {
    pub fn render_ui(&mut self, ui: &mut egui::Ui, app_fs: &AppFileSystem) {
        if ui.button("New Project").clicked() {
            self.create_project_modal = Some(CreateProjectModal::new(ProjectCreationSource::Empty));
        }

        ui.add_enabled_ui(!self.should_disable_ui(), |ui| {
            self.open_or_import_project.render_ui(ui);
            if let Some(project_id) = self.recent_projects_state.render_ui(ui, app_fs) {
                self.open_project(app_fs.clone(), project_id, vec![]);
            }
        });

        ui.heading("Featured Projects");
        for featured_project in FEATURED_PROJECTS {
            if ui.button(featured_project.name).clicked() {
                self.create_project_modal =
                    Some(CreateProjectModal::from_featured_project(featured_project));
            }
        }

        if let Some(modal) = &mut self.create_project_modal {
            if let Some(response) = modal.render_ui(ui) {
                match response {
                    CreateProjectModalResponse::Create { project_id, files } => {
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
        self.open_workspace_job = Some(AsyncJob::new(workspace_job));
    }

    pub fn render(
        &mut self,
        app_event_queue: &mut EventQueue<AppEvent>,
        app_file_system: &AppFileSystem,
    ) {
        self.recent_projects_state.tick(app_file_system);

        if let Some(result) = self.open_or_import_project.tick() {
            match result {
                #[cfg(target_arch = "wasm32")]
                Ok(project_import) => {
                    let project_id = project_import.project_id;
                    let files = project_import.files;
                    self.open_project(app_file_system.clone(), project_id, files);
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ok(project_id) => {
                    self.open_project(app_file_system.clone(), project_id, vec![]);
                }
                Err(error) => {
                    log::error!("Failed to pick project folder: {error:?}");
                    self.recent_projects_state.reload();
                }
            }
        }

        if let Some(job) = &mut self.open_workspace_job {
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
                self.open_workspace_job = None;
            }
        }
    }

    fn should_disable_ui(&self) -> bool {
        self.open_workspace_job.is_some() || self.open_or_import_project.is_picker_opened()
    }
}
