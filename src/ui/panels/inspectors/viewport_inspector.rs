use egui::Grid;

use crate::{
    project::ViewportId,
    ui::{components::selector::selectable_value_storage, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn viewport_inspector_ui(&mut self, ui: &mut egui::Ui, viewport_id: ViewportId) {
        let Ok(viewport) = self.project.viewports.get_mut(viewport_id) else {
            ui.label("Viewport couldn't be found.");
            return;
        };

        Grid::new("viewport_inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Texture View");
                let mut texture_view_id = Some(viewport.texture_view_id());
                let texture_view_id_before = texture_view_id;
                selectable_value_storage(
                    ui,
                    "viewport_texture_view",
                    &mut texture_view_id,
                    |_, texture_view| texture_view.label(),
                    &self.project.texture_views,
                );
                ui.end_row();
                if texture_view_id != texture_view_id_before {
                    viewport.set_texture_view_id(texture_view_id.unwrap());
                }

                ui.label("Dimension");
                let mut dimension_id = Some(viewport.dimension_id);
                let dimension_id_before = dimension_id;
                selectable_value_storage(
                    ui,
                    "viewport_dimension",
                    &mut dimension_id,
                    |_, dimension| dimension.label.as_str(),
                    &self.project.dimensions,
                );
                ui.end_row();
                if dimension_id != dimension_id_before {
                    viewport.dimension_id = dimension_id.unwrap();
                }

                ui.label("Controls Camera");
                let mut camera_id = Some(viewport.controls_camera_id);
                let camera_id_before = camera_id;
                selectable_value_storage(
                    ui,
                    "viewport_camera",
                    &mut camera_id,
                    |_, camera| camera.label.as_str(),
                    &self.project.cameras,
                );
                ui.end_row();
                if camera_id != camera_id_before {
                    viewport.controls_camera_id = camera_id.unwrap();
                }
            });
    }
}
