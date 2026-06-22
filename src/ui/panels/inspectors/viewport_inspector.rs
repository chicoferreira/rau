use crate::{
    project::ViewportId,
    ui::{
        components::{field, field_docs::field_doc, inspector},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn viewport_inspector_ui(&mut self, ui: &mut egui::Ui, viewport_id: ViewportId) {
        let Ok(viewport) = self.project.viewports.get_mut(viewport_id) else {
            ui.label("Viewport couldn't be found.");
            return;
        };

        inspector::section(ui, "Settings", |ui| {
            field::field_grid(ui, "viewport_inspector_grid", |ui| {
                let mut texture_view_id = viewport.texture_view_id();
                if field::row_doc(
                    ui,
                    "Texture View",
                    field_doc!("The Texture View this viewport displays."),
                    |ui| {
                        inspector::storage_combo(
                            ui,
                            "viewport_texture_view",
                            &self.project.texture_views,
                            &mut texture_view_id,
                        )
                    },
                ) {
                    viewport.set_texture_view_id(texture_view_id);
                }

                let mut dimension_id = viewport.dimension_id();
                if field::row_doc(
                    ui,
                    "Dimension",
                    field_doc!(
                        "The Dimension resource this viewport publishes its current size to. \
                        Textures and cameras that reference the same Dimension resize to match \
                        this viewport.\n\n\
                        Optional. Leave as **None** to not track the size."
                    ),
                    |ui| {
                        inspector::storage_opt_combo(
                            ui,
                            "viewport_dimension",
                            &self.project.dimensions,
                            &mut dimension_id,
                        )
                    },
                ) {
                    viewport.set_dimension_id(dimension_id);
                }

                let mut camera_id = viewport.controls_camera_id();
                if field::row_doc(
                    ui,
                    "Controls Camera",
                    field_doc!(
                        "The Camera this viewport drives. Mouse and keyboard input over the \
                        viewport moves that camera.\n\n\
                        Optional. Leave as **None** for a static view."
                    ),
                    |ui| {
                        inspector::storage_opt_combo(
                            ui,
                            "viewport_camera",
                            &self.project.cameras,
                            &mut camera_id,
                        )
                    },
                ) {
                    viewport.set_controls_camera_id(camera_id);
                }
            });
        });
    }
}
