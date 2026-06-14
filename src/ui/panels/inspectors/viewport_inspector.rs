use crate::{
    project::ViewportId,
    ui::{components::inspector, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn viewport_inspector_ui(&mut self, ui: &mut egui::Ui, viewport_id: ViewportId) {
        let Ok(viewport) = self.project.viewports.get_mut(viewport_id) else {
            ui.label("Viewport couldn't be found.");
            return;
        };

        inspector::section(ui, "Settings", |ui| {
            inspector::field_grid(ui, "viewport_inspector_grid", |ui| {
                let mut texture_view_id = viewport.texture_view_id();
                if inspector::storage_combo_row(
                    ui,
                    "Texture View",
                    "viewport_texture_view",
                    &self.project.texture_views,
                    &mut texture_view_id,
                ) {
                    viewport.set_texture_view_id(texture_view_id);
                }

                let mut dimension_id = viewport.dimension_id();
                if inspector::storage_opt_combo_row(
                    ui,
                    "Dimension",
                    "viewport_dimension",
                    &self.project.dimensions,
                    &mut dimension_id,
                ) {
                    viewport.set_dimension_id(dimension_id);
                }

                let mut camera_id = viewport.controls_camera_id();
                if inspector::storage_opt_combo_row(
                    ui,
                    "Controls Camera",
                    "viewport_camera",
                    &self.project.cameras,
                    &mut camera_id,
                ) {
                    viewport.set_controls_camera_id(camera_id);
                }
            });
        });
    }
}
