use crate::{
    project::{
        BindGroupId, CameraId, DimensionId, ModelId, ProjectResourceId, RenderPassId, SamplerId,
        ShaderId, TextureId, TextureViewId, UniformId, ViewportId,
    },
    ui::{components::tiles::Pane, pane::StateSnapshot},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPane {
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
}

impl InspectorPane {
    pub fn id(&self) -> ProjectResourceId {
        match *self {
            InspectorPane::Uniform(id) => id.into(),
            InspectorPane::BindGroup(id) => id.into(),
            InspectorPane::Shader(id) => id.into(),
            InspectorPane::Camera(id) => id.into(),
            InspectorPane::Dimension(id) => id.into(),
            InspectorPane::Sampler(id) => id.into(),
            InspectorPane::Texture(id) => id.into(),
            InspectorPane::TextureView(id) => id.into(),
            InspectorPane::Viewport(id) => id.into(),
            InspectorPane::Model(id) => id.into(),
            InspectorPane::RenderPass(id) => id.into(),
        }
    }
}

impl Pane for InspectorPane {
    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        let id = self.id();
        let label = state
            .project
            .label(id)
            .map(|l| l.to_string())
            .unwrap_or_else(|| format!("Unknown {:?}", id));

        label.into()
    }

    fn pane_ui(
        &mut self,
        state: &mut StateSnapshot<'_>,
        ui: &mut egui::Ui,
    ) -> egui_tiles::UiResponse {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                ui.push_id(self.id(), |ui| {
                    match self {
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
                    };
                });
            });
        });

        egui_tiles::UiResponse::None
    }
}
