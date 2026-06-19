use std::task::Poll;

use egui::{Ui, Widget};

use crate::error::{AppError, AppResult};
use crate::featured_projects::FeaturedProject;
#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
use crate::file::file_system::{AppFileSystem, FutureResult};
use crate::file::identifier::{ProjectIdentifier, ProjectSource};
use crate::project::{Project, paths::FilePath};
use crate::ui::components::inspector;
#[cfg(not(target_arch = "wasm32"))]
use crate::utils::async_job::AsyncJob;
use crate::utils::github;
use crate::utils::github::download::DownloadTask;

pub struct CreateProjectModal {
    form_data: CreateProjectFormData,
    state: CreateProjectModalState,
}

struct CreateProjectFormData {
    kind: ProjectCreationKind,
    storage: ProjectCreationStorage,
    github_source: GithubProjectSource,
    project_name: String,
    error: Option<AppError>,
    #[cfg(not(target_arch = "wasm32"))]
    project_path: Option<AbsolutePathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    folder_picker_job: Option<FutureResult<Option<AbsolutePathBuf>>>,
}

enum CreateProjectModalState {
    Editing,
    CheckingAvailability(ProjectAvailabilityTask),
    Downloading(ProjectDownloadTask),
}

#[derive(Clone)]
pub enum ProjectCreationSource {
    Empty,
    Github(GithubProjectSource),
}

impl ProjectCreationSource {
    /// The project name to fall back to when none is provided (the repo name for
    /// GitHub sources, nothing for empty projects).
    pub fn default_project_name(&self) -> Option<String> {
        match self {
            Self::Empty => None,
            Self::Github(source) => Some(source.repo.clone()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectCreationKind {
    Empty,
    Github,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectCreationStorage {
    Persistent,
    Temporary,
}

#[derive(Clone, Default)]
pub struct GithubProjectSource {
    pub owner: String,
    pub repo: String,
    pub git_ref: String,
    pub path: String,
}

pub enum CreateProjectModalResponse {
    Create {
        source: ProjectSource,
        files: Vec<(FilePath, Vec<u8>)>,
    },
    Close,
}

struct ProjectDownloadTask {
    source: ProjectSource,
    task: github::download::DownloadTask,
}

enum PendingProjectCreation {
    Empty(ProjectSource),
    Github(ProjectSource, github::GitRepository, FilePath),
}

struct ProjectAvailabilityTask {
    pending_creation: PendingProjectCreation,
    task: FutureResult<()>,
}

impl PendingProjectCreation {
    fn source(&self) -> &ProjectSource {
        match self {
            Self::Empty(source) | Self::Github(source, ..) => source,
        }
    }
}

impl ProjectDownloadTask {
    fn new(source: ProjectSource, repository: github::GitRepository, path: FilePath) -> Self {
        let task = github::download_files_under_path(&repository, &path);
        Self { source, task }
    }
}

impl CreateProjectModal {
    pub fn new(source: ProjectCreationSource) -> Self {
        let (kind, github_source, project_name) = match source {
            ProjectCreationSource::Empty => (
                ProjectCreationKind::Empty,
                GithubProjectSource::default(),
                "Untitled Project".to_string(),
            ),
            ProjectCreationSource::Github(source) => {
                (ProjectCreationKind::Github, source, String::new())
            }
        };

        Self {
            form_data: CreateProjectFormData {
                kind,
                storage: ProjectCreationStorage::Persistent,
                github_source,
                project_name,
                error: None,
                #[cfg(not(target_arch = "wasm32"))]
                project_path: None,
                #[cfg(not(target_arch = "wasm32"))]
                folder_picker_job: None,
            },
            state: CreateProjectModalState::Editing,
        }
    }

    pub fn from_featured_project(project: &'static FeaturedProject) -> Self {
        let github_project_source = GithubProjectSource {
            owner: project.owner.to_string(),
            repo: project.repo.to_string(),
            git_ref: project.git_ref.to_string(),
            path: project.path.to_string(),
        };

        let mut modal = Self::new(ProjectCreationSource::Github(github_project_source));
        modal.form_data.project_name = project.id.to_string();
        modal
    }

    pub fn from_cli(
        app_file_system: &AppFileSystem,
        source: ProjectSource,
        github: GithubProjectSource,
    ) -> Self {
        let mut modal = Self::new(ProjectCreationSource::Github(github));
        modal.form_data.apply_source(source);
        modal.start_project_creation(app_file_system);
        modal
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        app_file_system: &AppFileSystem,
        toasts: &mut egui_notify::Toasts,
    ) -> Option<CreateProjectModalResponse> {
        let mut result = None;

        let response =
            egui::Modal::new(egui::Id::new("create_project_modal")).show(ui.ctx(), |ui| {
                let is_busy = !matches!(self.state, CreateProjectModalState::Editing);

                ui.heading("Creating new project");

                ui.add_enabled_ui(!is_busy, |ui| {
                    ui_form(ui, &mut self.form_data, toasts);
                });

                if let Some(error) = &self.form_data.error {
                    inspector::error_label(ui, error.to_string());
                }

                result = self.tick_pending_state(ui);

                if result.is_some() {
                    return;
                }

                ui.add_enabled_ui(self.can_create_project(), |ui| {
                    if ui.button("Create Project").clicked() {
                        self.start_project_creation(app_file_system);
                    }
                });
            });

        if result.is_none() && response.should_close() {
            result = Some(CreateProjectModalResponse::Close);
        }

        result
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn can_create_project(&self) -> bool {
        matches!(self.state, CreateProjectModalState::Editing)
            && (!self.form_data.requires_project_path() || self.form_data.project_path.is_some())
            && self.form_data.folder_picker_job.is_none()
    }

    #[cfg(target_arch = "wasm32")]
    fn can_create_project(&self) -> bool {
        matches!(self.state, CreateProjectModalState::Editing)
    }

    fn start_project_creation(&mut self, app_file_system: &AppFileSystem) {
        self.form_data.error = None;

        let pending_creation = match self.form_data.pending_creation() {
            Ok(pending_creation) => pending_creation,
            Err(error) => {
                self.form_data.error = Some(error);
                return;
            }
        };

        let task = app_file_system.ensure_project_can_be_created(pending_creation.source().clone());
        self.state = CreateProjectModalState::CheckingAvailability(ProjectAvailabilityTask {
            pending_creation,
            task,
        });
    }

    fn tick_pending_state(&mut self, ui: &mut Ui) -> Option<CreateProjectModalResponse> {
        match std::mem::replace(&mut self.state, CreateProjectModalState::Editing) {
            CreateProjectModalState::Editing => None,
            CreateProjectModalState::CheckingAvailability(mut availability_task) => {
                match availability_task.task.try_resolve() {
                    Poll::Pending => {
                        self.state =
                            CreateProjectModalState::CheckingAvailability(availability_task);
                        None
                    }
                    Poll::Ready(Ok(())) => {
                        self.create_available_project(availability_task.pending_creation)
                    }
                    Poll::Ready(Err(error)) => {
                        self.form_data.error = Some(error);
                        None
                    }
                }
            }
            CreateProjectModalState::Downloading(mut download_task) => {
                if !ui_download(ui, &mut download_task) {
                    self.state = CreateProjectModalState::Downloading(download_task);
                    return None;
                }

                self.finish_download(download_task)
            }
        }
    }

    fn create_available_project(
        &mut self,
        pending_creation: PendingProjectCreation,
    ) -> Option<CreateProjectModalResponse> {
        match pending_creation {
            PendingProjectCreation::Empty(source) => match Project::default().serialize() {
                Ok(bytes) => Some(CreateProjectModalResponse::Create {
                    source,
                    files: vec![(FilePath::project_json(), bytes)],
                }),
                Err(error) => {
                    self.form_data.error = Some(error);
                    None
                }
            },
            PendingProjectCreation::Github(source, repository, path) => {
                let task = ProjectDownloadTask::new(source, repository, path);
                self.state = CreateProjectModalState::Downloading(task);
                None
            }
        }
    }

    fn finish_download(
        &mut self,
        download_task: ProjectDownloadTask,
    ) -> Option<CreateProjectModalResponse> {
        let ProjectDownloadTask { source, task } = download_task;
        match task {
            DownloadTask::Done { files } => {
                Some(CreateProjectModalResponse::Create { source, files })
            }
            DownloadTask::Errored { error } => {
                self.form_data.error = Some(error);
                None
            }
            task => {
                let task = ProjectDownloadTask { source, task };
                self.state = CreateProjectModalState::Downloading(task);
                None
            }
        }
    }
}

impl CreateProjectFormData {
    fn pending_creation(&self) -> AppResult<PendingProjectCreation> {
        let source = self.create_project_source()?;

        match self.kind {
            ProjectCreationKind::Empty => Ok(PendingProjectCreation::Empty(source)),
            ProjectCreationKind::Github => {
                let (repository, path) = self.github_source.create()?;
                Ok(PendingProjectCreation::Github(source, repository, path))
            }
        }
    }

    fn apply_source(&mut self, source: ProjectSource) {
        match source {
            ProjectSource::Ephemeral { project_name } => {
                self.storage = ProjectCreationStorage::Temporary;
                self.project_name = project_name;
            }
            ProjectSource::Persistent(identifier) => {
                self.storage = ProjectCreationStorage::Persistent;
                self.project_name = identifier.project_name().to_string();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.project_path = Some(identifier.project_path().clone());
                }
            }
        }
    }

    fn create_project_source(&self) -> AppResult<ProjectSource> {
        let project_name = self.valid_project_name()?;

        match self.storage {
            ProjectCreationStorage::Persistent => Ok(ProjectSource::Persistent(
                self.create_project_identifier(project_name)?,
            )),
            ProjectCreationStorage::Temporary => Ok(ProjectSource::Ephemeral { project_name }),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn create_project_identifier(&self, project_name: String) -> AppResult<ProjectIdentifier> {
        let project_path = self.get_actual_project_path(&project_name)?;
        Ok(ProjectIdentifier::new(project_name, project_path))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_project_identifier(&self, project_name: String) -> AppResult<ProjectIdentifier> {
        Ok(ProjectIdentifier::new(project_name))
    }

    fn valid_project_name(&self) -> AppResult<String> {
        if self.project_name.trim().is_empty() {
            return Err(AppError::InvalidCreateProjectForm(
                "project name is required",
            ));
        }

        Ok(self.project_name.clone())
    }

    /// Returns the project path appended with the project name if the folder does not end with the project name.
    #[cfg(not(target_arch = "wasm32"))]
    fn get_actual_project_path(&self, project_name: &str) -> AppResult<AbsolutePathBuf> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or(AppError::InvalidCreateProjectForm(
                "project folder is required",
            ))?;
        let path = project_path.as_path_buf();
        if path.ends_with(project_name) {
            return Ok(project_path.clone());
        }

        let path_with_name = path.join(project_name);

        AbsolutePathBuf::new(path_with_name)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn requires_project_path(&self) -> bool {
        self.storage == ProjectCreationStorage::Persistent
    }
}

impl GithubProjectSource {
    fn create(&self) -> AppResult<(github::GitRepository, FilePath)> {
        let owner = required_field(&self.owner, "GitHub owner is required")?;
        let repo = required_field(&self.repo, "GitHub repository is required")?;
        let git_ref = required_field(&self.git_ref, "GitHub branch/Commit SHA is required")?;
        let path = FilePath::from_str(self.path.trim())?;
        let repository = github::GitRepository::new(owner, repo, git_ref);

        Ok((repository, path))
    }
}

fn required_field<'a>(value: &'a str, message: &'static str) -> AppResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidCreateProjectForm(message));
    }

    Ok(value)
}

fn ui_form(
    ui: &mut egui::Ui,
    form_data: &mut CreateProjectFormData,
    #[allow(unused)] toasts: &mut egui_notify::Toasts,
) {
    egui::Grid::new("create_project_form")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Source:");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut form_data.kind, ProjectCreationKind::Empty, "Empty");
                ui.selectable_value(
                    &mut form_data.kind,
                    ProjectCreationKind::Github,
                    "From GitHub",
                );
            });
            ui.end_row();

            if form_data.kind == ProjectCreationKind::Github {
                text_edit_row(
                    ui,
                    "Owner:",
                    &mut form_data.github_source.owner,
                    "chicoferreira",
                );
                text_edit_row(ui, "Repository:", &mut form_data.github_source.repo, "rau");
                text_edit_row(
                    ui,
                    "Branch/Commit SHA:",
                    &mut form_data.github_source.git_ref,
                    "main",
                );
                text_edit_row(
                    ui,
                    "Folder in repository:",
                    &mut form_data.github_source.path,
                    "optional folder path",
                );
            }
        });

    ui.separator();

    egui::Grid::new("create_project_form_name_and_folder")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Storage:");
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut form_data.storage,
                    ProjectCreationStorage::Persistent,
                    "Persistent",
                );
                ui.selectable_value(
                    &mut form_data.storage,
                    ProjectCreationStorage::Temporary,
                    "Temporary",
                );
            });
            ui.end_row();

            ui.label("Project Name:");
            ui.text_edit_singleline(&mut form_data.project_name);
            ui.end_row();

            #[cfg(not(target_arch = "wasm32"))]
            if form_data.requires_project_path() {
                ui.label("Project Folder:");
                folder_selector_controls_ui(ui, form_data, toasts);
                ui.end_row();
            }
        });
}

fn text_edit_row(ui: &mut Ui, label: &str, value: &mut String, hint: &str) {
    ui.label(label);
    egui::TextEdit::singleline(value).hint_text(hint).ui(ui);
    ui.end_row();
}

#[cfg(not(target_arch = "wasm32"))]
fn folder_selector_controls_ui(
    ui: &mut egui::Ui,
    form_data: &mut CreateProjectFormData,
    toasts: &mut egui_notify::Toasts,
) {
    if let Some(job) = &mut form_data.folder_picker_job
        && let Poll::Ready(result) = job.try_resolve()
    {
        match result {
            Ok(Some(project_path)) => form_data.project_path = Some(project_path),
            Ok(None) => {}
            Err(error) => {
                toasts_log_error!(toasts, "Failed to select project folder: {error}");
            }
        }
        form_data.folder_picker_job = None;
    }

    ui.horizontal(|ui| {
        let path_str = form_data
            .get_actual_project_path(&form_data.project_name)
            .ok()
            .map(|path| path.as_ref().display().to_string())
            .unwrap_or_else(|| "No folder selected".to_string());

        ui.label(path_str);

        let picker_pending = form_data.folder_picker_job.is_some();

        ui.add_enabled_ui(!picker_pending, |ui| {
            if ui.button("Choose Folder").clicked() {
                let current_path = form_data.project_path.clone();
                form_data.folder_picker_job = Some(AsyncJob::new(async move {
                    let dialog = rfd::AsyncFileDialog::new().set_title("Select Project Folder");

                    let dialog = if let Some(current_path) = current_path {
                        dialog.set_directory(current_path.as_ref())
                    } else {
                        dialog
                    };

                    let Some(folder) = dialog.pick_folder().await else {
                        return Ok(None);
                    };

                    AbsolutePathBuf::try_from(folder.path().to_path_buf()).map(Some)
                }));
            }
        });

        if picker_pending {
            ui.spinner();
        }
    });
}

fn ui_download(ui: &mut Ui, download_task: &mut ProjectDownloadTask) -> bool {
    download_task.task.tick();

    if let Some((downloaded, total)) = download_task.task.file_count_progress() {
        let progress = if total == 0 {
            0.0
        } else {
            downloaded as f32 / total as f32
        };
        let text = format!("Downloading {downloaded} / {total} files...");
        ui.add(egui::ProgressBar::new(progress).text(text));
    } else {
        ui.add(egui::ProgressBar::new(0.0).text("Listing files..."));
    }

    matches!(
        download_task.task,
        DownloadTask::Done { .. } | DownloadTask::Errored { .. }
    )
}
