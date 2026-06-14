use crate::{
    project::{ShaderId, paths::FilePath},
    ui::{components::inspector, pane::StateSnapshot},
    utils::wgpu_utils::ShaderSourceKind,
};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        let Ok(shader) = self.project.shaders.get_mut(shader_id) else {
            return;
        };

        inspector::section(ui, "Source", |ui| {
            inspector::field_grid(ui, "shader_inspector_grid", |ui| {
                let mut source = shader.source().cloned();
                let Some(files) = self.file_storage.files() else {
                    ui.spinner();
                    return;
                };

                let is_shader_source = |path: &FilePath| {
                    path.extension()
                        .and_then(ShaderSourceKind::from_extension)
                        .is_some()
                };

                if inspector::file_combo_row(
                    ui,
                    "Source",
                    "shader_source",
                    &files,
                    &mut source,
                    is_shader_source,
                ) {
                    shader.set_source(source);
                }
            });
        });
    }
}
