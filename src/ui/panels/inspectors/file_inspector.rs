use crate::{
    file::file_storage::OpenFileState,
    project::paths::FilePath,
    ui::{components::code_editor::code_editor, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn file_inspector_ui(&mut self, ui: &mut egui::Ui, file_path: &FilePath) {
        match self.file_storage.open_file(file_path) {
            OpenFileState::Loading { .. } => {
                ui.horizontal(|ui| {
                    ui.label("Loading...");
                    ui.spinner();
                });
            }
            OpenFileState::Loaded { text, saved_text }
            | OpenFileState::Reloading {
                text, saved_text, ..
            } => {
                if code_editor(ui, text).changed() && text != saved_text {
                    let bytes = text.clone().into_bytes();
                    self.file_storage.save_in_background(file_path, bytes);
                }
            }
            OpenFileState::Errored { error } => {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        };
    }
}
