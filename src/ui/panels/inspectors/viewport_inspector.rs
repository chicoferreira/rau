use egui::Grid;

use crate::{
    project::ViewportId,
    ui::{components::selector::ComboBoxExt, pane::StateSnapshot},
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
                let mut texture_view_id = viewport.texture_view_id;
                let texture_view_id_before = texture_view_id;
                let texture_views = &self.project.texture_views;
                egui::ComboBox::from_id_salt("viewport_texture_view")
                    .selected_text_storage_opt(texture_views, texture_view_id)
                    .show_ui_storage_opt_with_none(ui, texture_views, &mut texture_view_id);

                ui.end_row();
                if texture_view_id != texture_view_id_before {
                    viewport.texture_view_id = texture_view_id;
                }

                ui.label("Dimension");
                let mut dimension_id = viewport.dimension_id;
                let dimension_id_before = dimension_id;
                egui::ComboBox::from_id_salt("viewport_dimension")
                    .selected_text_storage_opt(&self.project.dimensions, dimension_id)
                    .show_ui_storage_opt_with_none(ui, &self.project.dimensions, &mut dimension_id);
                ui.end_row();
                if dimension_id != dimension_id_before {
                    viewport.dimension_id = dimension_id;
                }

                ui.label("Controls Camera");
                let mut camera_id = viewport.controls_camera_id;
                let camera_id_before = camera_id;
                egui::ComboBox::from_id_salt("viewport_camera")
                    .selected_text_storage_opt(&self.project.cameras, camera_id)
                    .show_ui_storage_opt_with_none(ui, &self.project.cameras, &mut camera_id);
                ui.end_row();
                if camera_id != camera_id_before {
                    viewport.controls_camera_id = camera_id;
                }
            });
    }
}
