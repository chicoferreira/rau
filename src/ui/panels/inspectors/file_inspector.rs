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
