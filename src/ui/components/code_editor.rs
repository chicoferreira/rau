pub fn code_editor(ui: &mut egui::Ui, text: &mut String) -> egui::Response {
    ui.add(
        egui::TextEdit::multiline(text)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(24)
            .desired_width(f32::INFINITY)
            .lock_focus(true),
    )
}
