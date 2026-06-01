use crate::file::identifier::ProjectIdentifier;

pub struct DeleteProjectConfirmationModal {
    project_id: ProjectIdentifier,
}

pub enum DeleteProjectConfirmationModalResponse {
    Confirm(ProjectIdentifier),
    Cancel,
}

impl DeleteProjectConfirmationModal {
    pub fn new(project_id: ProjectIdentifier) -> Self {
        Self { project_id }
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
    ) -> Option<DeleteProjectConfirmationModalResponse> {
        let mut result = None;

        let response = egui::Modal::new(egui::Id::new("delete_project_confirmation_modal")).show(
            ui.ctx(),
            |ui| {
                let project_name = self.project_id.project_name();

                let title = cfg_select! {
                    target_arch = "wasm32" => "Delete Project?",
                    _ => "Remove Recent Project?",
                };
                let message = cfg_select! {
                    target_arch = "wasm32" => format!("Delete \"{project_name}\"? This cannot be undone."),
                    _ => format!("Remove \"{project_name}\" from Recent Projects? The project files will stay on disk.",),
                };
                let confirm_button_label = cfg_select! {
                    target_arch = "wasm32" => "Delete Project",
                    _ => "Remove",
                };

                ui.heading(title);
                ui.label(message);

                ui.horizontal(|ui| {
                    if ui.button(confirm_button_label).clicked() {
                        result = Some(DeleteProjectConfirmationModalResponse::Confirm(
                            self.project_id.clone(),
                        ));
                    }

                    if ui.button("Cancel").clicked() {
                        result = Some(DeleteProjectConfirmationModalResponse::Cancel);
                    }
                });
            },
        );

        if result.is_none() && response.should_close() {
            result = Some(DeleteProjectConfirmationModalResponse::Cancel);
        }

        result
    }
}
