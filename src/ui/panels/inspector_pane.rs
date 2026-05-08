use std::fmt::Debug;

use crate::{
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, FramePlanId, ModelId, RenderPassId,
        ResourceId, SamplerId, ShaderId, TextureId, TextureViewId, UniformId, ViewportId,
        paths::FilePath,
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
    RenderPass(RenderPassId),
    FramePlan(FramePlanId),
    ComputePass(ComputePassId),
}

fn resource_tab_title(id: impl Into<ResourceId>, state: &StateSnapshot<'_>) -> String {
    let id = id.into();
    let label_opt = state.project.label(id).map(|l| l.to_string());
    label_opt.unwrap_or_else(|| format!("Unknown {:?}", id))
}

impl Pane for InspectorPane {
    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        match self {
            InspectorPane::File(file_path) => file_path.to_string().into(),
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
            InspectorPane::RenderPass(id) => resource_tab_title(*id, state).into(),
            InspectorPane::FramePlan(id) => resource_tab_title(*id, state).into(),
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
                            todo!("{:?}", file_path);
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
                        InspectorPane::RenderPass(render_pass_id) => {
                            state.render_pass_inspector_ui(ui, *render_pass_id);
                        }
                        InspectorPane::FramePlan(_) => {
                            state.frame_plan_inspector_ui(ui);
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
