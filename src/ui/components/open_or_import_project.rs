use std::task::Poll;

#[cfg(target_arch = "wasm32")]
use crate::error::AppError;
use crate::{error::AppResult, file::identifier::ProjectIdentifier, utils::async_job::AsyncJob};

#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
#[cfg(target_arch = "wasm32")]
use crate::project::paths::FilePath;
#[cfg(target_arch = "wasm32")]
use crate::utils::browser::folder_picker;

#[derive(Default)]
pub struct OpenOrImportProject {
    job: Option<AsyncJob<AppResult<Option<OpenOrImportProjectJobResult>>>>,
}

#[cfg(target_arch = "wasm32")]
pub type OpenOrImportProjectJobResult = ProjectImport;
#[cfg(not(target_arch = "wasm32"))]
pub type OpenOrImportProjectJobResult = ProjectIdentifier;

#[cfg(target_arch = "wasm32")]
pub struct ProjectImport {
    pub project_id: ProjectIdentifier,
    pub files: Vec<(FilePath, Vec<u8>)>,
}

impl OpenOrImportProject {
    pub fn render_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let is_picker_opened = self.job.is_some();
            ui.add_enabled_ui(!is_picker_opened, |ui| {
                #[cfg(target_arch = "wasm32")]
                if ui.button("Import Project").clicked() {
                    self.job = Some(import_project_from_folder());
                }

                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Open Project").clicked() {
                    self.job = Some(open_project_from_folder());
                }
            });

            if is_picker_opened {
                ui.spinner();
            }
        });
    }

    pub fn tick(&mut self) -> Option<AppResult<OpenOrImportProjectJobResult>> {
        let Some(job) = &mut self.job else {
            return None;
        };

        let Poll::Ready(result) = job.try_resolve() else {
            return None;
        };

        self.job = None;

        result.transpose()
    }

    pub fn is_picker_opened(&self) -> bool {
        self.job.is_some()
    }
}

#[cfg(target_arch = "wasm32")]
fn import_project_from_folder() -> AsyncJob<AppResult<Option<ProjectImport>>> {
    AsyncJob::new(async move {
        let Some((project_name, files)) = folder_picker::pick_folder_files().await? else {
            return Ok(None);
        };

        if !files.iter().any(|(path, _)| path.is_project_json()) {
            return Err(AppError::MissingProjectJson);
        }

        Ok(Some(ProjectImport {
            project_id: ProjectIdentifier::new(project_name),
            files,
        }))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn open_project_from_folder() -> AsyncJob<AppResult<Option<ProjectIdentifier>>> {
    AsyncJob::new(async move {
        let Some(folder) = rfd::AsyncFileDialog::new()
            .set_title("Open Project Folder")
            .pick_folder()
            .await
        else {
            return Ok(None);
        };

        let project_path = AbsolutePathBuf::try_from(folder.path().to_path_buf())?;
        Ok(Some(ProjectIdentifier::extract_identifier(project_path)?))
    })
}
