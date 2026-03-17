use egui::RichText;

pub fn ui_array_mut<const N: usize>(
    ui: &mut egui::Ui,
    array: &mut [f32; N],
    widget: impl Fn(&mut egui::Ui, &mut f32) -> bool,
) -> bool {
    let mut changed = false;
    for value in array.iter_mut() {
        changed |= widget(ui, value);
    }
    changed
}

pub fn ui_array<const N: usize>(
    ui: &mut egui::Ui,
    array: &[f32; N],
    widget: impl Fn(&mut egui::Ui, &f32),
) {
    for value in array {
        widget(ui, value);
    }
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
