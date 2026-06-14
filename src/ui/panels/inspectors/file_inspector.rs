use crate::{
    file::file_storage::OpenFileState,
    project::paths::FilePath,
    ui::{
        components::{code_editor::code_editor, inspector},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn file_inspector_ui(&mut self, ui: &mut egui::Ui, file_path: &FilePath) {
        match self.file_storage.open_file(file_path) {
            OpenFileState::Loading { .. } => {
                inspector::centered(ui, |ui| {
                    ui.add(egui::Spinner::new().size(ui.text_style_height(&egui::TextStyle::Body)));
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
                inspector::centered_error(ui, format!("Couldn't open file:\n{error}"));
            }
        };
    }
}
