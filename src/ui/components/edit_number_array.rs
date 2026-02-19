pub fn ui_edit_array_h<T: egui::emath::Numeric, const N: usize>(
    ui: &mut egui::Ui,
    array: &mut [T; N],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= ui_edit_array(ui, array);
    });
    changed
}

pub fn ui_edit_array<T: egui::emath::Numeric, const N: usize>(
    ui: &mut egui::Ui,
    array: &mut [T; N],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for value in array.iter_mut() {
            changed |= ui.add(egui::DragValue::new(value).speed(0.01)).changed();
        }
    });
    changed
}
