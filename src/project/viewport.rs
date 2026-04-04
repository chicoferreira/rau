use crate::{
    error::AppResult,
    project::{CameraId, DimensionId, ProjectResource, TextureViewId},
    ui::Size2d,
};

pub struct Viewport {
    pub label: String,
    pub texture_view_id: Option<TextureViewId>,
    pub dimension_id: Option<DimensionId>,
    pub controls_camera_id: Option<CameraId>,
    pub requested_ui_size: Option<Size2d>,
}

#[allow(dead_code)]
impl Viewport {
    pub fn new(
        label: impl Into<String>,
        texture_view_id: Option<TextureViewId>,
        dimension_id: Option<DimensionId>,
        controls_camera_id: Option<CameraId>,
    ) -> AppResult<Viewport> {
        let name = label.into();

        Ok(Viewport {
            label: name,
            texture_view_id,
            dimension_id,
            controls_camera_id,
            requested_ui_size: None,
        })
    }
}

impl ProjectResource for Viewport {
    fn label(&self) -> &str {
        &self.label
    }
}
