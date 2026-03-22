use crate::{
    error::SourcedError,
    project::{self},
    state,
    ui::{
        components::tiles::TreePane,
        panels::{
            error_panel, inspector_pane::InspectorPane, project_tree_panel,
            viewport_pane::ViewportPane,
        },
        rename::RenameState,
    },
};

pub struct StateSnapshot<'a> {
    pub pending_events: &'a mut Vec<state::StateEvent>,
    pub project: &'a mut project::Project,
    pub rename_state: &'a mut Option<RenameState>,
    pub errors: &'a Vec<SourcedError>,
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

        egui::Panel::left("project_tree_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    project_tree_panel::ui(self, ui);
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
