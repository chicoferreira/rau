use crate::{
    project::{ShaderId, paths::FilePath},
    ui::{components::inspector, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        let Some(files) = self.file_storage.files().map(|files| files.to_vec()) else {
            ui.spinner();
            return;
        };

        let Ok(shader) = self.project.shaders.get_mut(shader_id) else {
            return;
        };

        inspector::field_grid(ui, "shader_inspector_grid", |ui| {
            let mut source = shader.source().cloned();
            if inspector::file_opt_combo_row(
                ui,
                "Source",
                "shader_source",
                &files,
                &mut source,
                is_wgsl_file,
                |path| path.to_string().into(),
            ) {
                shader.set_source(source);
            }
        });
    }
}

fn is_wgsl_file(path: &FilePath) -> bool {
    path.extension() == Some("wgsl")
}
