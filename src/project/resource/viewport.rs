use crate::{
    project::{CameraId, Creatable, DimensionId, ProjectResource, TextureViewId, ViewportId},
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
    ) -> Viewport {
        let name = label.into();

        Viewport {
            label: name,
            texture_view_id,
            dimension_id,
            controls_camera_id,
            requested_ui_size: None,
        }
    }
}

impl Creatable for Viewport {
    fn create(label: String) -> Self {
        Viewport::new(label, None, None, None)
    }
}

impl ProjectResource for Viewport {
    type Id = ViewportId;

    fn label(&self) -> &str {
        &self.label
    }
}
