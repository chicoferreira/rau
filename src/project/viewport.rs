use crate::{
    error::AppResult,
    project::{
        CameraId, DimensionId, TextureViewId, ViewportId,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
        storage::Storage,
        texture_view::TextureView,
    },
    ui::{self, Size2d},
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
    pub requested_ui_size: Option<Size2d>,
    pub controls_camera_id: CameraId,
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
        controls_camera_id: CameraId,
    ) -> AppResult<Viewport> {
        let name = label.into();

        let texture_view = context.texture_views.get(texture_view_id)?;

        let egui_id = context.egui_renderer.register_egui_texture(
            context.device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
        );

        Ok(Viewport {
            label: name,
            texture_view_id,
            dimension_id,
            controls_camera_id,
            egui_id,
            requested_ui_size: None,
            dirty: false,
        })
    }

    pub fn egui_id(&self) -> egui::TextureId {
        self.egui_id
    }
}

impl Recreatable for Viewport {
    type Context<'a> = ViewportCreationContext<'a>;
    type Id = ViewportId;

    fn recreate<'a>(
        &mut self,
        _id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        if !self.dirty
            && !tracker.happened(ProjectEvent::TextureViewRecreated(self.texture_view_id))
        {
            return Ok(None);
        }

        let texture_view = ctx.texture_views.get(self.texture_view_id)?;
        self.dirty = false;

        ctx.egui_renderer.update_egui_texture(
            ctx.device,
            texture_view.inner(),
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
        Ok(None)
    }
}
