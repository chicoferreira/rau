use crate::{
    project::{
        DimensionId, TextureViewId,
        recreate::{Recreatable, RecreateResult, RecreateTracker},
        storage::Storage,
        texture_view::TextureView,
    },
    ui::{self},
};

pub struct ViewportCreationContext<'a> {
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
    pub egui_renderer: &'a mut ui::renderer::EguiRenderer,
    pub device: &'a wgpu::Device,
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
        context: ViewportCreationContext,
        label: impl Into<String>,
        texture_view_id: TextureViewId,
        dimension_id: DimensionId,
    ) -> Viewport {
        let name = label.into();

        let texture_view = context
            .texture_views
            .get(texture_view_id)
            .expect("deal with this later");

        let egui_id = context.egui_renderer.register_egui_texture(
            context.device,
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

    pub fn egui_id(&self) -> egui::TextureId {
        self.egui_id
    }
}

impl Recreatable for Viewport {
    type Context<'a> = ViewportCreationContext<'a>;

    fn recreate<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> RecreateResult {
        if !self.dirty && !tracker.was_recreated(self.texture_view_id) {
            return RecreateResult::Unchanged;
        }
        let Some(texture_view) = ctx.texture_views.get(self.texture_view_id) else {
            return RecreateResult::Unchanged;
        };

        ctx.egui_renderer.update_egui_texture(
            ctx.device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
            self.egui_id,
        );

        RecreateResult::Recreated
    }
}
