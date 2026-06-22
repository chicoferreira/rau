use std::task::Poll;

use egui::{Ui, Widget};
use egui_phosphor::regular;

use crate::error::{AppError, AppResult};
#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
use crate::file::file_system::{AppFileSystem, FutureResult};
use crate::file::identifier::{ProjectIdentifier, ProjectSource};
use crate::project::{Project, paths::FilePath};
use crate::ui::components::field;
use crate::ui::components::field_docs::field_doc;
use crate::ui::components::main_menu::featured_projects::FeaturedProject;
use crate::ui::components::main_menu::menu_widgets;
use crate::ui::components::resource_icons;
#[cfg(not(target_arch = "wasm32"))]
use crate::utils::async_job::AsyncJob;
use crate::utils::github;
use crate::utils::github::download::DownloadTask;

const MODAL_WIDTH: f32 = 460.0;

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

        let frame = egui::Frame::popup(ui.style()).inner_margin(20);
        let response = egui::Modal::new(egui::Id::new("create_project_modal"))
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(MODAL_WIDTH);

                let is_busy = !matches!(self.state, CreateProjectModalState::Editing);

                menu_widgets::modal_title(
                    ui,
                    "Create New Project",
                    "Start from a blank canvas or pull an existing project from GitHub.",
                );
                ui.add_space(6.0);

                ui.add_enabled_ui(!is_busy, |ui| {
                    ui_form(ui, &mut self.form_data, toasts);
                });

                if let Some(error) = &self.form_data.error {
                    ui.add_space(6.0);
                    field::error_label(ui, error.to_string());
                }

                if let Some(pending) = self.tick_pending_state(ui) {
                    result = Some(pending);
                    return;
                }

                ui.add_space(14.0);

                ui.horizontal(|ui| {
                    let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                    if menu_widgets::action_button_sized(ui, "Cancel", egui::vec2(half, 34.0))
                        .clicked()
                    {
                        result = Some(CreateProjectModalResponse::Close);
                    }

                    let size = egui::vec2(ui.available_width(), 34.0);
                    ui.add_enabled_ui(self.can_create_project(), |ui| {
                        let label = resource_icons::monochrome_icon_text(
                            ui,
                            regular::PLUS,
                            egui::Color32::WHITE,
                            "Create Project",
                        );
                        if menu_widgets::primary_action_button_sized(ui, label, size).clicked() {
                            self.start_project_creation(app_file_system);
                        }
                    });
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
    menu_widgets::modal_section_header(ui, "Source");

    form_grid(ui, "create_project_form", |ui| {
        // The source kind sits in the grid so it lines up with the GitHub fields
        // below it.
        field::row_doc(
            ui,
            "Source Type",
            field_doc!(
                "Choose **Empty** to start from a blank project, or **From GitHub** \
                 to download an existing project from a repository."
            ),
            |ui| {
                ui.horizontal(|ui| {
                    source_selector(
                        ui,
                        &mut form_data.kind,
                        ProjectCreationKind::Empty,
                        regular::FILE_DASHED,
                        "Empty",
                    );
                    source_selector(
                        ui,
                        &mut form_data.kind,
                        ProjectCreationKind::Github,
                        regular::GITHUB_LOGO,
                        "From GitHub",
                    );
                });
            },
        );

        if form_data.kind == ProjectCreationKind::Github {
            field::row_doc(
                ui,
                "Owner",
                field_doc!("The GitHub user or organization that owns the repository."),
                |ui| text_edit(ui, &mut form_data.github_source.owner, "chicoferreira"),
            );
            field::row_doc(
                ui,
                "Repository",
                field_doc!("The name of the GitHub repository to download the project from."),
                |ui| text_edit(ui, &mut form_data.github_source.repo, "rau"),
            );
            field::row_doc(
                ui,
                "Branch / Commit",
                field_doc!("The branch name or commit SHA to download the project at."),
                |ui| text_edit(ui, &mut form_data.github_source.git_ref, "main"),
            );
            field::row_doc(
                ui,
                "Folder in repository",
                field_doc!(
                    "Optional path to the project folder inside the repository. \
                     Leave empty to use the repository root."
                ),
                |ui| {
                    text_edit(
                        ui,
                        &mut form_data.github_source.path,
                        "optional folder path",
                    )
                },
            );
        }
    });

    ui.add_space(6.0);

    menu_widgets::modal_section_header(ui, "Details");

    form_grid(ui, "create_project_form_name_and_folder", |ui| {
        field::row_doc(
            ui,
            "Storage",
            field_doc!(
                "**Persistent** projects are saved to disk. **Temporary** projects \
                 are kept only in memory and are lost when the app closes."
            ),
            |ui| {
                ui.horizontal(|ui| {
                    source_selector(
                        ui,
                        &mut form_data.storage,
                        ProjectCreationStorage::Persistent,
                        regular::FLOPPY_DISK,
                        "Persistent",
                    );
                    source_selector(
                        ui,
                        &mut form_data.storage,
                        ProjectCreationStorage::Temporary,
                        regular::HOURGLASS,
                        "Temporary",
                    );
                });
            },
        );

        field::row_doc(
            ui,
            "Project Name",
            field_doc!("The name of the new project."),
            |ui| text_edit(ui, &mut form_data.project_name, ""),
        );

        #[cfg(not(target_arch = "wasm32"))]
        if form_data.requires_project_path() {
            field::row_doc(
                ui,
                "Project Folder",
                field_doc!("The folder on disk where the project will be created."),
                |ui| folder_selector_controls_ui(ui, form_data, toasts),
            );
        }
    });
}

fn form_grid<R>(ui: &mut Ui, id_salt: &str, add_rows: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Grid::new(id_salt)
        .num_columns(2)
        .spacing(egui::vec2(12.0, 8.0))
        .show(ui, add_rows)
        .inner
}

fn text_edit(ui: &mut Ui, value: &mut String, hint: &str) {
    egui::TextEdit::singleline(value)
        .hint_text(hint)
        .desired_width(f32::INFINITY)
        .ui(ui);
}

fn source_selector<T: PartialEq>(
    ui: &mut Ui,
    current: &mut T,
    value: T,
    glyph: &'static str,
    label: &str,
) {
    let selected = *current == value;
    let text = resource_icons::monochrome_icon_text(ui, glyph, ui.visuals().text_color(), label);
    if ui.add(egui::Button::selectable(selected, text)).clicked() {
        *current = value;
    }
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

    let picker_pending = form_data.folder_picker_job.is_some();

    let selected_path = form_data
        .get_actual_project_path(&form_data.project_name)
        .ok()
        .map(|path| path.as_ref().display().to_string());

    let (label, label_color) = match &selected_path {
        Some(path) => (path.as_str(), ui.visuals().text_color()),
        None => ("Choose a folder…", ui.visuals().error_fg_color),
    };

    ui.horizontal(|ui| {
        if picker_pending {
            ui.spinner();
        }

        let text = resource_icons::monochrome_icon_text(ui, regular::FOLDER, label_color, label);

        let button = egui::Button::new(text)
            .truncate()
            .right_text(())
            .min_size(egui::vec2(ui.available_width(), 0.0));

        let response = ui.add_enabled(!picker_pending, button);

        if response.clicked() {
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

        if let Some(path) = &selected_path {
            response.on_hover_text(path);
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
