use egui::RichText;

pub fn ui_edit_array<const N: usize>(
    ui: &mut egui::Ui,
    array: &mut [f32; N],
    widget: impl Fn(&mut egui::Ui, &mut f32),
) {
    for value in array.iter_mut() {
        widget(ui, value);
    }
}

pub fn drag_value_widget(ui: &mut egui::Ui, value: &mut f32) {
    ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(2));
}

pub fn label_widget(ui: &mut egui::Ui, value: &mut f32) {
    ui.label(RichText::new(format!("{value:.2}")).weak());
}

pub fn ui_mat4_grid(ui: &mut egui::Ui, mat: &[[f32; 4]; 4]) {
    egui::Grid::new(ui.id().with("mat4grid"))
        .min_col_width(40.0)
        .show(ui, |ui| {
            for row in mat.iter() {
                for value in row.iter() {
                    ui.label(RichText::new(format!("{value:.3}")).weak().monospace());
                }
                ui.end_row();
            }
        });
}
