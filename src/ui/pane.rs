use crate::{
    file_storage::FileStorage,
    project::{Project, RuntimeProject},
    state,
    ui::{
        components::tiles::TreePane,
        panels::{
            error_panel, files_panel, inspector_pane::InspectorPane, project_tree_panel,
            viewport_pane::ViewportPane,
        },
        rename::RenameState,
    },
};

pub struct StateSnapshot<'a> {
    pub pending_events: &'a mut Vec<state::StateEvent>,
    pub project: &'a mut Project,
    pub runtime_project: &'a mut RuntimeProject,
    pub rename_state: &'a mut Option<RenameState>,
    pub file_storage: &'a mut FileStorage,
}

impl StateSnapshot<'_> {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        inspector_tree_pane: &mut TreePane<InspectorPane>,
        viewport_tree_pane: &mut TreePane<ViewportPane>,
    ) {
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Rau", |ui| {
                    if ui.button("Settings").clicked() {}
                    if ui.button("Quit").clicked() {}
                });

                ui.menu_button("Project", |ui| if ui.button("New").clicked() {});
            });
        });

        error_panel::ui(self, ui);

        egui::Panel::left("left_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                let half_height = ui.available_height() * 0.5;

                egui::Panel::top("files_panel")
                    .resizable(true)
                    .default_size(half_height)
                    .show_inside(ui, |ui| {
                        ui.take_available_height();

                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                files_panel::ui(self, ui);
                            });
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(0))
                    .show_inside(ui, |ui| {
                        ui.take_available_height();

                        egui::ScrollArea::both()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                project_tree_panel::ui(self, ui);
                            });
                    });
            });

        egui::Panel::right("inspector_tree_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                inspector_tree_pane.ui(self, ui);
            });

        viewport_tree_pane.ui(self, ui);
    }
}
