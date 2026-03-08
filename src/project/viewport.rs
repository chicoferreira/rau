use crate::{
    project::{
        DimensionId, TextureViewId,
        recreate::{Recreatable, RecreateResult, RecreateTracker},
        storage::Storage,
        texture_view::TextureView,
    },
    ui::{self},
};

pub struct ViewportContext<'a> {
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
    pub egui_renderer: &'a mut ui::renderer::EguiRenderer,
}

pub struct Viewport {
    pub label: String,
    pub texture_view_id: TextureViewId,
    pub dimension_id: DimensionId,
    egui_id: egui::TextureId,
    dirty: bool,
}

#[allow(dead_code)]
impl Viewport {
    pub fn new(
        context: ViewportContext,
        label: impl Into<String>,
        device: &wgpu::Device,
        texture_view_id: TextureViewId,
        dimension_id: DimensionId,
    ) -> Viewport {
        let name = label.into();

        let texture_view = context
            .texture_views
            .get(texture_view_id)
            .expect("deal with this later");

        let egui_id = context.egui_renderer.register_egui_texture(
            device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
        );

        Viewport {
            label: name,
            texture_view_id,
            dimension_id,
            egui_id,
            dirty: false,
        }
    }

    fn update(
        &mut self,
        project: ViewportContext,
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

impl Recreatable for Viewport {
    type Context<'a> = ViewportContext<'a>;

    fn recreate<'a>(
        &mut self,
        context: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> RecreateResult {
        if !self.dirty && !tracker.was_recreated(self.texture_view_id) {
            return RecreateResult::Unchanged;
        }
        let Some(texture_view) = context.texture_views.get(self.texture_view_id) else {
            return RecreateResult::Unchanged;
        };

        context.egui_renderer.update_egui_texture(
            device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
            self.egui_id,
        );

        RecreateResult::Recreated
    }
}
