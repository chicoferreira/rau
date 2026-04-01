use crate::{
    project::{
        BindGroupId, CameraId, DimensionId, ModelId, SamplerId, ShaderId, TextureId, TextureViewId,
        UniformId, ViewportId,
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
}

impl Pane for InspectorPane {
    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        let label = match self {
            InspectorPane::Uniform(id) => state.project.label(*id),
            InspectorPane::BindGroup(id) => state.project.label(*id),
            InspectorPane::Shader(id) => state.project.label(*id),
            InspectorPane::Camera(id) => state.project.label(*id),
            InspectorPane::Dimension(id) => state.project.label(*id),
            InspectorPane::Sampler(id) => state.project.label(*id),
            InspectorPane::Texture(id) => state.project.label(*id),
            InspectorPane::TextureView(id) => state.project.label(*id),
            InspectorPane::Model(id) => state.project.label(*id),
            InspectorPane::Viewport(id) => state.project.label(*id),
        };

        label
            .map(|l| l.to_string())
            .unwrap_or(format!("Unknown {:?}", self))
            .into()
    }

    fn pane_ui(
        &mut self,
        state: &mut StateSnapshot<'_>,
        ui: &mut egui::Ui,
    ) -> egui_tiles::UiResponse {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
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
                };
            });
        });

        egui_tiles::UiResponse::None
    }
}
