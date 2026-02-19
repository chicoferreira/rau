use crate::{
    project::{self},
    state,
    ui::panels::{
        inspector_pane::InspectorTreePane, project_tree_panel, viewport_pane::ViewportTreePane,
    },
};

pub struct StateSnapshot<'a> {
    pub pending_events: &'a mut Vec<state::StateEvent>,
    pub project: &'a mut project::Project,
    pub queue: &'a wgpu::Queue,
}

impl StateSnapshot<'_> {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        inspector_tree_pane: &mut InspectorTreePane,
        viewport_tree_pane: &mut ViewportTreePane,
    ) {
        egui::Panel::left("project_tree_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                project_tree_panel::ui(self, ui, inspector_tree_pane, viewport_tree_pane);
            });

        egui::Panel::right("inspector_tree_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.take_available_space();
                inspector_tree_pane.ui(self, ui);
            });

        viewport_tree_pane.ui(self, ui);
    }
}
