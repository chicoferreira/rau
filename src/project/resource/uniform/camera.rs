use crate::project::resource::camera::{Camera, CameraRuntime};
use crate::project::resource::uniform::UniformFieldData;

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum CameraField {
    Position,
    Projection,
    View,
    #[strum(to_string = "Projection View")]
    ProjectionView,
    #[strum(to_string = "Inverse Projection")]
    InverseProjection,
    #[strum(to_string = "Inverse View")]
    InverseView,
}

impl CameraField {
    pub(super) fn compute(
        &self,
        camera: &Camera,
        camera_runtime: &CameraRuntime,
    ) -> UniformFieldData {
        let position = camera.position();
        let matrix = camera_runtime.matrix();
        match self {
            CameraField::Position => UniformFieldData::Vec4f(position.to_homogeneous().into()),
            CameraField::Projection => UniformFieldData::Mat4x4f(matrix.projection.into()),
            CameraField::View => UniformFieldData::Mat4x4f(matrix.view.into()),
            CameraField::ProjectionView => UniformFieldData::Mat4x4f(matrix.projection_view.into()),
            CameraField::InverseProjection => UniformFieldData::Mat4x4f(matrix.inv_proj.into()),
            CameraField::InverseView => UniformFieldData::Mat4x4f(matrix.inverse_view.into()),
        }
    }
}
