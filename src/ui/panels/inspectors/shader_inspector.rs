use crate::{
    project::ShaderId,
    ui::{components::inspector, pane::StateSnapshot},
};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        let Ok(shader) = self.project.shaders.get_mut(shader_id) else {
            return;
        };

        inspector::field_grid(ui, "shader_inspector_grid", |ui| {
            let mut source = shader.source().cloned();
            let Some(files) = self.file_storage.files() else {
                ui.spinner();
                return;
            };

            if inspector::file_opt_combo_row(
                ui,
                "Source",
                "shader_source",
                &files,
                &mut source,
                |path| path.extension() == Some("wgsl"),
            ) {
                shader.set_source(source);
            }
        });
    }
}
