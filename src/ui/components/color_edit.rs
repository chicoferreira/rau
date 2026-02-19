pub fn color_edit_rgba(ui: &mut egui::Ui, color: &mut [f32; 4]) -> bool {
    let mut egui_color =
        egui::Rgba::from_rgba_premultiplied(color[0], color[1], color[2], color[3]);

    let color_picker = egui::color_picker::color_edit_button_rgba(
        ui,
        &mut egui_color,
        egui::color_picker::Alpha::OnlyBlend,
    );

    if color_picker.changed() {
        *color = egui_color.to_array();
        true
    } else {
        false
    }
}
