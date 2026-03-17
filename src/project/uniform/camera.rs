use crate::project::{camera::Camera, uniform::UniformFieldData};

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
    pub(super) fn default_data(&self) -> UniformFieldData {
        match self {
            CameraField::Projection => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::Position => UniformFieldData::Vec4f([0.0; 4]),
            CameraField::View => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::ProjectionView => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::InverseProjection => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::InverseView => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
        }
    }

    pub(super) fn compute(&self, camera: &Camera) -> UniformFieldData {
        match self {
            CameraField::Position => {
                UniformFieldData::Vec4f(camera.position().to_homogeneous().into())
            }
            CameraField::Projection => UniformFieldData::Mat4x4f(camera.matrix().projection.into()),
            CameraField::View => UniformFieldData::Mat4x4f(camera.matrix().view.into()),
            CameraField::ProjectionView => {
                UniformFieldData::Mat4x4f(camera.matrix().projection_view.into())
            }
            CameraField::InverseProjection => {
                UniformFieldData::Mat4x4f(camera.matrix().inverse_projection.into())
            }
            CameraField::InverseView => {
                UniformFieldData::Mat4x4f(camera.matrix().inverse_view.into())
            }
        }
    }
}
