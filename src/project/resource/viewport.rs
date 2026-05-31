use serde::{Deserialize, Serialize};

use crate::{
    project::{
        CameraId, Creatable, DimensionId, ProjectResource, TextureViewId, ViewportId,
        sync::Revision,
    },
    resource_getters, resource_setters,
    ui::size::Size2d,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    label: String,
    texture_view_id: Option<TextureViewId>,
    dimension_id: Option<DimensionId>,
    controls_camera_id: Option<CameraId>,
    #[serde(skip)]
    requested_ui_size: Option<Size2d>,
    #[serde(skip)]
    project_revision: Revision,
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
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn texture_view_id() -> Option<TextureViewId>;
        pub fn dimension_id() -> Option<DimensionId>;
        pub fn controls_camera_id() -> Option<CameraId>;
        pub fn requested_ui_size() -> Option<Size2d>;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_label(label: String);
        pub fn set_texture_view_id(texture_view_id: Option<TextureViewId>);
        pub fn set_dimension_id(dimension_id: Option<DimensionId>);
        pub fn set_controls_camera_id(controls_camera_id: Option<CameraId>);
        pub fn set_requested_ui_size(requested_ui_size: Option<Size2d>);
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

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}
