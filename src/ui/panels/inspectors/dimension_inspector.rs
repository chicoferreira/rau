use egui::{Grid, Widget};

use crate::{
    project::DimensionId,
    ui::{Size2d, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn dimension_inspector_ui(&mut self, ui: &mut egui::Ui, dimension_id: DimensionId) {
        let Ok(dimension) = self.project.dimensions.get_mut(dimension_id) else {
            ui.label("Dimension couldn't be found.");
            return;
        };

        let mut width = dimension.size.width();
        let mut height = dimension.size.height();

        Grid::new("dimension_inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Width");
                egui::DragValue::new(&mut width)
                    .speed(1)
                    .range(1_u32..=u32::MAX)
                    .ui(ui);
                ui.end_row();

                ui.label("Height");
                egui::DragValue::new(&mut height)
                    .speed(1)
                    .range(1_u32..=u32::MAX)
                    .ui(ui);
                ui.end_row();
            });

        dimension.size = Size2d::new(width, height);
    }
}
