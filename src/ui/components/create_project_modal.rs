use egui::{RichText, Ui};

use crate::error::AppError;
#[cfg(not(target_arch = "wasm32"))]
use crate::error::AppResult;
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
    source: ProjectCreationSource,
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

#[derive(Clone, Copy)]
pub enum ProjectCreationSource {
    Empty,
    Featured(&'static FeaturedProject),
}

pub enum CreateProjectModalResponse {
    Create {
        project_identifier: ProjectIdentifier,
        files: Vec<(FilePath, Vec<u8>)>,
    },
    Close,
}

struct ProjectDownloadTask {
    project_identifier: ProjectIdentifier,
    task: github::download::DownloadTask,
}

impl CreateProjectModal {
    pub fn new(source: ProjectCreationSource) -> Self {
        let project_name = match source {
            ProjectCreationSource::Empty => "Untitled Project",
            ProjectCreationSource::Featured(project) => project.id,
        };
        let project_name = project_name.to_string();

        Self {
            form_data: CreateProjectFormData {
                source,
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

    pub fn render_ui(&mut self, ui: &mut egui::Ui) -> Option<CreateProjectModalResponse> {
        let mut result = None;

        let response =
            egui::Modal::new(egui::Id::new("create_project_modal")).show(ui.ctx(), |ui| {
                let is_downloading = matches!(self.state, CreateProjectModalState::Downloading(_));

                ui_title(ui, &self.form_data);

                ui.add_enabled_ui(!is_downloading, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Project Name:");
                        ui.text_edit_singleline(&mut self.form_data.project_name);
                    });

                    #[cfg(not(target_arch = "wasm32"))]
                    folder_selector_ui(ui, &mut self.form_data);
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
                                    project_identifier: download_task.project_identifier,
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

                    let Some(project_identifier) = self.form_data.create_project_identifier()
                    else {
                        return;
                    };

                    self.form_data.error = None;

                    match self.form_data.source {
                        ProjectCreationSource::Empty => match Project::default().serialize() {
                            Ok(bytes) => {
                                result = Some(CreateProjectModalResponse::Create {
                                    project_identifier,
                                    files: vec![(FilePath::project_json(), bytes)],
                                });
                            }
                            Err(error) => {
                                self.form_data.error = Some(error);
                            }
                        },
                        ProjectCreationSource::Featured(featured_project) => {
                            match FilePath::from_str(featured_project.path) {
                                Ok(path) => {
                                    let repository = featured_project.repository();
                                    let task =
                                        github::download_files_under_path(&repository, &path);

                                    let download_task = ProjectDownloadTask {
                                        project_identifier,
                                        task,
                                    };
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
    fn create_project_identifier(&self) -> Option<ProjectIdentifier> {
        let project_name = self.project_name.clone();
        let project_path = self.get_actual_project_path()?;
        Some(ProjectIdentifier::new(project_name, project_path))
    }

    #[cfg(target_arch = "wasm32")]
    fn create_project_identifier(&self) -> Option<ProjectIdentifier> {
        Some(ProjectIdentifier::new(self.project_name.clone()))
    }

    /// Returns the project path appended with the project name if the folder does not end with the project name.
    #[cfg(not(target_arch = "wasm32"))]
    fn get_actual_project_path(&self) -> Option<AbsolutePathBuf> {
        let path = self.project_path.as_ref()?.as_path_buf();
        if path.ends_with(&self.project_name) {
            return self.project_path.clone();
        }

        let path_with_name = path.join(&self.project_name);

        AbsolutePathBuf::new(path_with_name).ok()
    }
}

fn ui_title(ui: &mut egui::Ui, form_data: &CreateProjectFormData) {
    match form_data.source {
        ProjectCreationSource::Empty => {
            ui.heading("Creating new empty project");
        }
        ProjectCreationSource::Featured(project) => {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.heading("Creating new project based of ");
                ui.label(RichText::new(project.name).heading().strong());
            });
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn folder_selector_ui(ui: &mut egui::Ui, form_data: &mut CreateProjectFormData) {
    if let Some(job) = &mut form_data.folder_picker_job {
        if let std::task::Poll::Ready(result) = job.try_resolve() {
            match result {
                Ok(Some(project_path)) => form_data.project_path = Some(project_path),
                Ok(None) => {}
                Err(error) => {
                    log::error!("Failed to select project folder: {error}");
                }
            }
            form_data.folder_picker_job = None;
        }
    }

    ui.horizontal(|ui| {
        ui.label("Project Folder:");

        let path_str = form_data
            .get_actual_project_path()
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
