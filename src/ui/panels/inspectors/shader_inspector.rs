use crate::{project::ShaderId, ui::pane::StateSnapshot};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        ui.take_available_space();
        let Ok(_shader) = self.project.shaders.get_mut(shader_id) else {
            return;
        };

        // TODO: add a file selector
    }
}
