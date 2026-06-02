use std::fmt::Debug;

use crate::{
    file::file_storage::OpenFileState,
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, ModelId, PresentationId, RenderPassId,
        RenderPipelineId, ResourceId, SamplerId, ShaderId, TextureId, TextureViewId, UniformId,
        ViewportId, paths::FilePath,
    },
    ui::{components::tiles::Pane, pane::StateSnapshot},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InspectorPane {
    File(FilePath),
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Shader(ShaderId),
    Camera(CameraId),
    Dimension(DimensionId),
    Sampler(SamplerId),
    Texture(TextureId),
    TextureView(TextureViewId),
    Viewport(ViewportId),
    Model(ModelId),
    RenderPipeline(RenderPipelineId),
    RenderPass(RenderPassId),
    Presentation(PresentationId),
    ComputePass(ComputePassId),
}

fn resource_tab_title(id: impl Into<ResourceId>, state: &StateSnapshot<'_>) -> String {
    let id = id.into();
    let label_opt = state.project.label(id).map(|l| l.to_string());
    label_opt.unwrap_or_else(|| format!("Unknown {:?}", id))
}

fn file_tab_title(file_path: &FilePath, state: &StateSnapshot<'_>) -> String {
    // TODO: add loading indicator in case of loading/reloading
    match state.file_storage.get_open_file(file_path) {
        Some(
            OpenFileState::Loaded { text, saved_text }
            | OpenFileState::Reloading {
                text, saved_text, ..
            },
        ) if text != saved_text => {
            format!("{} *", file_path)
        }
        _ => file_path.to_string(),
    }
}

impl Pane for InspectorPane {
    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        match self {
            InspectorPane::File(file_path) => file_tab_title(file_path, state).into(),
            InspectorPane::Uniform(id) => resource_tab_title(*id, state).into(),
            InspectorPane::BindGroup(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Shader(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Camera(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Dimension(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Sampler(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Texture(id) => resource_tab_title(*id, state).into(),
            InspectorPane::TextureView(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Viewport(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Model(id) => resource_tab_title(*id, state).into(),
            InspectorPane::RenderPipeline(id) => resource_tab_title(*id, state).into(),
            InspectorPane::RenderPass(id) => resource_tab_title(*id, state).into(),
            InspectorPane::Presentation(id) => resource_tab_title(*id, state).into(),
            InspectorPane::ComputePass(id) => resource_tab_title(*id, state).into(),
        }
    }

    fn pane_ui(
        &mut self,
        state: &mut StateSnapshot<'_>,
        ui: &mut egui::Ui,
    ) -> egui_tiles::UiResponse {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.push_id(self.clone(), |ui| {
                    match self {
                        InspectorPane::File(file_path) => {
                            state.file_inspector_ui(ui, file_path);
                        }
                        InspectorPane::Uniform(uniform_id) => {
                            state.uniform_inspector_ui(*uniform_id, ui);
                        }
                        InspectorPane::BindGroup(bind_group_id) => {
                            state.bind_group_inspector_ui(*bind_group_id, ui);
                        }
                        InspectorPane::Shader(shader_id) => {
                            state.shader_inspector_ui(ui, *shader_id);
                        }
                        InspectorPane::Camera(camera_id) => {
                            state.camera_inspector_ui(ui, *camera_id);
                        }
                        InspectorPane::Dimension(dimension_id) => {
                            state.dimension_inspector_ui(ui, *dimension_id);
                        }
                        InspectorPane::Sampler(sampler_id) => {
                            state.sampler_inspector_ui(ui, *sampler_id);
                        }
                        InspectorPane::Texture(texture_id) => {
                            state.texture_inspector_ui(ui, *texture_id);
                        }
                        InspectorPane::TextureView(texture_view_id) => {
                            state.texture_view_inspector_ui(ui, *texture_view_id)
                        }
                        InspectorPane::Viewport(viewport_id) => {
                            state.viewport_inspector_ui(ui, *viewport_id);
                        }
                        InspectorPane::Model(model_id) => {
                            state.model_inspector_ui(ui, *model_id);
                        }
                        InspectorPane::RenderPipeline(render_pipeline_id) => {
                            state.render_pipeline_inspector_ui(ui, *render_pipeline_id);
                        }
                        InspectorPane::RenderPass(render_pass_id) => {
                            state.render_pass_inspector_ui(ui, *render_pass_id);
                        }
                        InspectorPane::Presentation(_) => {
                            state.presentation_inspector_ui(ui);
                        }
                        InspectorPane::ComputePass(compute_pass_id) => {
                            state.compute_pass_inspector_ui(ui, *compute_pass_id);
                        }
                    };
                });
            });
        });

        egui_tiles::UiResponse::None
    }
}
