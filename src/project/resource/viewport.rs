use serde::{Deserialize, Serialize};

use crate::{
    project::{
        CameraId, Creatable, DimensionId, ProjectResource, TextureViewId, ViewportId,
        sync::Revision,
    },
    ui::Size2d,
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

    pub fn set_label(&mut self, label: String) {
        if self.label != label {
            self.label = label;
            self.project_revision.increase();
        }
    }

    pub fn texture_view_id(&self) -> Option<TextureViewId> {
        self.texture_view_id
    }

    pub fn set_texture_view_id(&mut self, texture_view_id: Option<TextureViewId>) {
        if self.texture_view_id != texture_view_id {
            self.texture_view_id = texture_view_id;
            self.project_revision.increase();
        }
    }

    pub fn dimension_id(&self) -> Option<DimensionId> {
        self.dimension_id
    }

    pub fn set_dimension_id(&mut self, dimension_id: Option<DimensionId>) {
        if self.dimension_id != dimension_id {
            self.dimension_id = dimension_id;
            self.project_revision.increase();
        }
    }

    pub fn controls_camera_id(&self) -> Option<CameraId> {
        self.controls_camera_id
    }

    pub fn set_controls_camera_id(&mut self, camera_id: Option<CameraId>) {
        if self.controls_camera_id != camera_id {
            self.controls_camera_id = camera_id;
            self.project_revision.increase();
        }
    }

    pub fn requested_ui_size(&self) -> Option<Size2d> {
        self.requested_ui_size
    }

    pub fn set_requested_ui_size(&mut self, size: Option<Size2d>) {
        if self.requested_ui_size != size {
            self.requested_ui_size = size;
            self.project_revision.increase();
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

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}
