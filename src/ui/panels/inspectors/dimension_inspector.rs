use crate::{
    project::DimensionId,
    ui::{components::inspector, pane::StateSnapshot, size::Size2d},
};

impl StateSnapshot<'_> {
    pub fn dimension_inspector_ui(&mut self, ui: &mut egui::Ui, dimension_id: DimensionId) {
        let Ok(dimension) = self.project.dimensions.get_mut(dimension_id) else {
            ui.label("Dimension couldn't be found.");
            return;
        };

        let mut width = dimension.size().width();
        let mut height = dimension.size().height();

        inspector::field_grid(ui, "dimension_inspector_grid", |ui| {
            inspector::u32_drag_row(ui, "Width", &mut width, 1_u32..=u32::MAX);
            inspector::u32_drag_row(ui, "Height", &mut height, 1_u32..=u32::MAX);
        });

        dimension.set_size(Size2d::new(width, height));
    }
}
