use crate::{
    project::{ShaderId, paths::FilePath},
    ui::{
        components::{field, field_docs::field_doc, inspector},
        pane::StateSnapshot,
    },
    utils::wgpu_utils::ShaderSourceKind,
};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        let Ok(shader) = self.project.shaders.get_mut(shader_id) else {
            return;
        };

        inspector::section(ui, "Source", |ui| {
            field::field_grid(ui, "shader_inspector_grid", |ui| {
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

                if field::row_doc(
                    ui,
                    "Source",
                    field_doc!(
                        "The source file that will be compiled into this shader.\n\n\
                        Supports **WGSL** (`.wgsl`) and **GLSL** (`.vert`, `.frag`, `.comp`).\n\n\
                        [WGSL spec](https://www.w3.org/TR/WGSL/) | \
                        [GLSL spec](https://registry.khronos.org/OpenGL/specs/gl/GLSLangSpec.4.60.html)"
                    ),
                    |ui| {
                        inspector::file_combo(
                            ui,
                            "shader_source",
                            files,
                            &mut source,
                            is_shader_source,
                        )
                    },
                ) {
                    shader.set_source(source);
                }
            });
        });
    }
}
