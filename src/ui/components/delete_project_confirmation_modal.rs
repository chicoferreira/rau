use egui::RichText;
use egui_phosphor::regular;

use crate::file::identifier::ProjectIdentifier;
use crate::ui::components::main_menu::menu_widgets;
use crate::ui::components::resource_icons;

/// Fixed width of the confirmation modal.
const MODAL_WIDTH: f32 = 400.0;

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

        let frame = egui::Frame::popup(ui.style()).inner_margin(20);
        let response = egui::Modal::new(egui::Id::new("delete_project_confirmation_modal"))
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(MODAL_WIDTH);

                let project_name = self.project_id.project_name();

                let title = cfg_select! {
                    target_arch = "wasm32" => "Delete Project?",
                    _ => "Remove Recent Project?",
                };
                let message = cfg_select! {
                    target_arch = "wasm32" => format!("\"{project_name}\" will be permanently deleted. This cannot be undone."),
                    _ => format!("\"{project_name}\" will be removed from Recent Projects. The project files will stay on disk."),
                };
                let confirm_button_label = cfg_select! {
                    target_arch = "wasm32" => "Delete Project",
                    _ => "Remove",
                };

                menu_widgets::modal_title(ui, title, "");
                ui.add_space(10.0);
                ui.label(RichText::new(message));
                ui.add_space(16.0);

                ui.horizontal(|ui| {
                    let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                    if menu_widgets::action_button_sized(ui, "Cancel", egui::vec2(half, 34.0))
                        .clicked()
                    {
                        result = Some(DeleteProjectConfirmationModalResponse::Cancel);
                    }

                    let size = egui::vec2(ui.available_width(), 34.0);
                    let confirm = resource_icons::monochrome_icon_text(
                        ui,
                        regular::TRASH,
                        egui::Color32::WHITE,
                        confirm_button_label,
                    );
                    if menu_widgets::danger_action_button_sized(ui, confirm, size).clicked() {
                        result = Some(DeleteProjectConfirmationModalResponse::Confirm(
                            self.project_id.clone(),
                        ));
                    }
                });
            });

        if result.is_none() && response.should_close() {
            result = Some(DeleteProjectConfirmationModalResponse::Cancel);
        }

        result
    }
}
