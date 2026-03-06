use crate::{
    project::{DimensionId, TextureViewId, storage::Storage, texture_view::TextureView},
    ui::{self},
};

pub struct ViewportProjectView<'a> {
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
}

pub struct Viewport {
    pub label: String,
    pub texture_view_id: TextureViewId,
    pub dimension_id: DimensionId,
    egui_id: egui::TextureId,
}

#[allow(dead_code)]
impl Viewport {
    pub fn new(
        project: ViewportProjectView,
        label: impl Into<String>,
        device: &wgpu::Device,
        texture_view_id: TextureViewId,
        dimension_id: DimensionId,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) -> Viewport {
        let name = label.into();

        let texture_view = project
            .texture_views
            .get(texture_view_id)
            .expect("deal with this later");

        let egui_id = egui_renderer.register_egui_texture(
            device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
        );

        Viewport {
            label: name,
            texture_view_id,
            dimension_id,
            egui_id,
        }
    }

    pub fn update(
        &mut self,
        project: ViewportProjectView,
        device: &wgpu::Device,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        let texture_view = project
            .texture_views
            .get(self.texture_view_id)
            .expect("deal with this later");

        egui_renderer.update_egui_texture(
            device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
    }

    pub fn egui_id(&self) -> egui::TextureId {
        self.egui_id
    }
}
