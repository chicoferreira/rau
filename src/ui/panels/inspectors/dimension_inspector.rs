use crate::{
    project::DimensionId,
    ui::{
        components::{field, field_docs::field_doc, inspector},
        pane::StateSnapshot,
        size::Size2d,
    },
};

impl StateSnapshot<'_> {
    pub fn dimension_inspector_ui(&mut self, ui: &mut egui::Ui, dimension_id: DimensionId) {
        let Ok(dimension) = self.project.dimensions.get_mut(dimension_id) else {
            ui.label("Dimension couldn't be found.");
            return;
        };

        let mut width = dimension.size().width();
        let mut height = dimension.size().height();

        inspector::section_doc(
            ui,
            "Size",
            field_doc!(
                "A named **size** (width and height) shared by other resources.\n\n\
                Textures, cameras and viewports that reference this Dimension all use this size \
                and update together when it changes, either edited here or driven by a viewport \
                bound to it."
            ),
            |ui| {
                field::field_grid(ui, "dimension_inspector_grid", |ui| {
                    inspector::u32_drag_row_doc(
                        ui,
                        "Width",
                        field_doc!("The Dimension's **width**, in pixels."),
                        &mut width,
                        1_u32..=u32::MAX,
                    );
                    inspector::u32_drag_row_doc(
                        ui,
                        "Height",
                        field_doc!("The Dimension's **height**, in pixels."),
                        &mut height,
                        1_u32..=u32::MAX,
                    );
                });
            },
        );

        dimension.set_size(Size2d::new(width, height));
    }
}
