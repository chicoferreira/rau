use crate::{
    file::file_storage::OpenFileState,
    project::paths::FilePath,
    ui::{components::code_editor::code_editor, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn file_inspector_ui(&mut self, ui: &mut egui::Ui, file_path: &FilePath) {
        match self.file_storage.open_file(file_path) {
            OpenFileState::Loading { .. } => {
                let row_height = ui.text_style_height(&egui::TextStyle::Body);
                let top_padding = (ui.available_height() - row_height).max(0.0) / 2.0;
                ui.add_space(top_padding);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new().size(row_height));
                        ui.label("Loading...");
                    });
                });
            }
            OpenFileState::Loaded { text, saved }
            | OpenFileState::Reloading { text, saved, .. } => {
                let extension = file_path.extension().unwrap_or("");
                if code_editor(ui, text, extension).changed() && text != &saved.text {
                    let contents = text.clone();
                    self.file_storage.save_open_file(file_path, contents);
                }
            }
            OpenFileState::Errored { error } => {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        };
    }
}
