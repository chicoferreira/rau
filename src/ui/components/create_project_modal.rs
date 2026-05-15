use crate::error::AppResult;
#[cfg(not(target_arch = "wasm32"))]
use crate::file::absolute::AbsolutePathBuf;
use crate::file::identifier::ProjectIdentifier;
use crate::utils::async_job::AsyncJob;

pub struct CreateProjectModal {
    project_name: String,
    #[cfg(not(target_arch = "wasm32"))]
    project_path: Option<AbsolutePathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    folder_picker_job: Option<AsyncJob<AppResult<Option<AbsolutePathBuf>>>>,
}

impl Default for CreateProjectModal {
    fn default() -> Self {
        Self {
            project_name: "Untitled Project".to_string(),
            #[cfg(not(target_arch = "wasm32"))]
            project_path: None,
            #[cfg(not(target_arch = "wasm32"))]
            folder_picker_job: None,
        }
    }
}

impl CreateProjectModal {
    pub fn render_ui(&mut self, ui: &mut egui::Ui) -> Option<ProjectIdentifier> {
        let mut result = None;

        egui::Modal::new(egui::Id::new("create_project_modal")).show(ui.ctx(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Project Name:");
                ui.text_edit_singleline(&mut self.project_name);
            });

            #[cfg(not(target_arch = "wasm32"))]
            folder_selector_ui(ui, self);

            ui.add_enabled_ui(self.can_create_project(), |ui| {
                if ui.button("Create Project").clicked() {
                    result = self.create_project_identifier();
                }
            });
        });

        result
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn can_create_project(&self) -> bool {
        self.project_path.is_some() && self.folder_picker_job.is_none()
    }

    #[cfg(target_arch = "wasm32")]
    fn can_create_project(&self) -> bool {
        true
    }

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

#[cfg(not(target_arch = "wasm32"))]
fn folder_selector_ui(ui: &mut egui::Ui, modal: &mut CreateProjectModal) {
    if let Some(job) = &mut modal.folder_picker_job {
        if let std::task::Poll::Ready(result) = job.try_resolve() {
            match result {
                Ok(Some(project_path)) => modal.project_path = Some(project_path),
                Ok(None) => {}
                Err(error) => {
                    log::error!("Failed to select project folder: {error}");
                }
            }
            modal.folder_picker_job = None;
        }
    }

    ui.horizontal(|ui| {
        ui.label("Project Folder:");

        let path_str = modal
            .get_actual_project_path()
            .map(|path| path.as_ref().display().to_string())
            .unwrap_or_else(|| "No folder selected".to_string());

        ui.label(path_str);

        let picker_pending = modal.folder_picker_job.is_some();

        ui.add_enabled_ui(!picker_pending, |ui| {
            if ui.button("Choose Folder").clicked() {
                let current_path = modal.project_path.clone();
                modal.folder_picker_job = Some(AsyncJob::new(async move {
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
