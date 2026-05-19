use egui::{Ui, Widget};

use crate::error::{AppError, AppResult};
use crate::featured_projects::FeaturedProject;
#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
use crate::file::identifier::ProjectIdentifier;
use crate::project::{Project, paths::FilePath};
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
    github_source: GithubProjectSource,
    project_name: String,
    error: Option<AppError>,
    #[cfg(not(target_arch = "wasm32"))]
    project_path: Option<AbsolutePathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    folder_picker_job: Option<AsyncJob<AppResult<Option<AbsolutePathBuf>>>>,
}

enum CreateProjectModalState {
    Editing,
    Downloading(ProjectDownloadTask),
}

#[derive(Clone)]
pub enum ProjectCreationSource {
    Empty,
    Github(GithubProjectSource),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProjectCreationKind {
    Empty,
    Github,
}

#[derive(Clone, Default)]
pub struct GithubProjectSource {
    owner: String,
    repo: String,
    git_ref: String,
    path: String,
}

pub enum CreateProjectModalResponse {
    Create {
        project_id: ProjectIdentifier,
        files: Vec<(FilePath, Vec<u8>)>,
    },
    Close,
}

struct ProjectDownloadTask {
    project_id: ProjectIdentifier,
    task: github::download::DownloadTask,
}

impl CreateProjectModal {
    pub fn new(source: ProjectCreationSource) -> Self {
        let project_name = match &source {
            ProjectCreationSource::Empty => "Untitled Project".to_string(),
            ProjectCreationSource::Github(_) => "".to_string(),
        };

        let kind = match source {
            ProjectCreationSource::Empty => ProjectCreationKind::Empty,
            ProjectCreationSource::Github(_) => ProjectCreationKind::Github,
        };

        let github_source = match source {
            ProjectCreationSource::Empty => GithubProjectSource::default(),
            ProjectCreationSource::Github(source) => source,
        };

        Self {
            form_data: CreateProjectFormData {
                kind,
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

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        toasts: &mut egui_notify::Toasts,
    ) -> Option<CreateProjectModalResponse> {
        let mut result = None;

        let response =
            egui::Modal::new(egui::Id::new("create_project_modal")).show(ui.ctx(), |ui| {
                let is_downloading = matches!(self.state, CreateProjectModalState::Downloading(_));

                ui.heading("Creating new project");

                ui.add_enabled_ui(!is_downloading, |ui| {
                    ui_form(ui, &mut self.form_data, toasts);
                });

                if let Some(error) = &self.form_data.error {
                    ui.colored_label(ui.visuals().error_fg_color, error.to_string());
                }

                let download_finished = ui_download(ui, &mut self.state);

                if download_finished {
                    let state =
                        std::mem::replace(&mut self.state, CreateProjectModalState::Editing);
                    if let CreateProjectModalState::Downloading(download_task) = state {
                        match download_task.task {
                            DownloadTask::Done { files } => {
                                result = Some(CreateProjectModalResponse::Create {
                                    project_id: download_task.project_id,
                                    files,
                                });
                            }
                            DownloadTask::Errored { error } => {
                                self.form_data.error = Some(error);
                            }
                            _ => {}
                        }
                    }
                }

                ui.add_enabled_ui(self.can_create_project(), |ui| {
                    if !ui.button("Create Project").clicked() {
                        return;
                    }

                    self.form_data.error = None;

                    let project_id = match self.form_data.create_project_identifier() {
                        Ok(id) => id,
                        Err(error) => {
                            self.form_data.error = Some(error);
                            return;
                        }
                    };

                    match self.form_data.kind {
                        ProjectCreationKind::Empty => match Project::default().serialize() {
                            Ok(bytes) => {
                                result = Some(CreateProjectModalResponse::Create {
                                    project_id,
                                    files: vec![(FilePath::project_json(), bytes)],
                                });
                            }
                            Err(error) => {
                                self.form_data.error = Some(error);
                            }
                        },
                        ProjectCreationKind::Github => {
                            match self.form_data.github_source.create() {
                                Ok((repository, path)) => {
                                    let task =
                                        github::download_files_under_path(&repository, &path);

                                    let download_task = ProjectDownloadTask { project_id, task };
                                    self.state =
                                        CreateProjectModalState::Downloading(download_task);
                                }
                                Err(error) => {
                                    self.form_data.error = Some(error);
                                }
                            }
                        }
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
            && self.form_data.project_path.is_some()
            && self.form_data.folder_picker_job.is_none()
    }

    #[cfg(target_arch = "wasm32")]
    fn can_create_project(&self) -> bool {
        matches!(self.state, CreateProjectModalState::Editing)
    }
}

impl CreateProjectFormData {
    #[cfg(not(target_arch = "wasm32"))]
    fn create_project_identifier(&self) -> AppResult<ProjectIdentifier> {
        let project_name = self.valid_project_name()?;
        let project_path = self.get_actual_project_path()?;
        Ok(ProjectIdentifier::new(project_name, project_path))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_project_identifier(&self) -> AppResult<ProjectIdentifier> {
        let project_name = self.valid_project_name()?;
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
    fn get_actual_project_path(&self) -> AppResult<AbsolutePathBuf> {
        let project_path = self
            .project_path
            .as_ref()
            .ok_or(AppError::InvalidCreateProjectForm(
                "project folder is required",
            ))?;
        let path = project_path.as_path_buf();
        if path.ends_with(&self.project_name) {
            return Ok(project_path.clone());
        }

        let path_with_name = path.join(&self.project_name);

        AbsolutePathBuf::new(path_with_name)
    }
}

impl GithubProjectSource {
    fn create(&self) -> AppResult<(github::GitRepository, FilePath)> {
        let owner = self.owner.trim();
        if owner.is_empty() {
            return Err(AppError::InvalidCreateProjectForm(
                "GitHub owner is required",
            ));
        }

        let repo = self.repo.trim();
        if repo.is_empty() {
            return Err(AppError::InvalidCreateProjectForm(
                "GitHub repository is required",
            ));
        }

        let git_ref = self.git_ref.trim();
        if git_ref.is_empty() {
            return Err(AppError::InvalidCreateProjectForm(
                "GitHub branch/Commit SHA is required",
            ));
        }

        let path = FilePath::from_str(self.path.trim())?;
        let repository = github::GitRepository::new(owner, repo, git_ref);
        Ok((repository, path))
    }
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
                ui.label("Owner:");
                egui::TextEdit::singleline(&mut form_data.github_source.owner)
                    .hint_text("chicoferreira")
                    .ui(ui);
                ui.end_row();

                ui.label("Repository:");
                egui::TextEdit::singleline(&mut form_data.github_source.repo)
                    .hint_text("rau")
                    .ui(ui);
                ui.end_row();

                ui.label("Branch/Commit SHA:");
                egui::TextEdit::singleline(&mut form_data.github_source.git_ref)
                    .hint_text("main")
                    .ui(ui);
                ui.end_row();

                ui.label("Folder in repository:");
                egui::TextEdit::singleline(&mut form_data.github_source.path)
                    .hint_text("optional folder path")
                    .ui(ui);
                ui.end_row();
            }
        });

    ui.separator();

    egui::Grid::new("create_project_form_name_and_folder")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Project Name:");
            ui.text_edit_singleline(&mut form_data.project_name);
            ui.end_row();

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.label("Project Folder:");
                folder_selector_controls_ui(ui, form_data, toasts);
                ui.end_row();
            }
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn folder_selector_controls_ui(
    ui: &mut egui::Ui,
    form_data: &mut CreateProjectFormData,
    toasts: &mut egui_notify::Toasts,
) {
    if let Some(job) = &mut form_data.folder_picker_job {
        if let std::task::Poll::Ready(result) = job.try_resolve() {
            match result {
                Ok(Some(project_path)) => form_data.project_path = Some(project_path),
                Ok(None) => {}
                Err(error) => {
                    toasts_log_error!(toasts, "Failed to select project folder: {error}");
                }
            }
            form_data.folder_picker_job = None;
        }
    }

    ui.horizontal(|ui| {
        let path_str = form_data
            .get_actual_project_path()
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

fn ui_download(ui: &mut Ui, state: &mut CreateProjectModalState) -> bool {
    if let CreateProjectModalState::Downloading(download_task) = state {
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
    } else {
        false
    }
}
