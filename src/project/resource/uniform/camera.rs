use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::project::resource::camera::{Camera, CameraRuntime};
use crate::project::resource::uniform::UniformFieldData;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Display, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
            CameraField::Position => UniformFieldData::Vec4f(position.extend(1.0).to_array()),
            CameraField::Projection => {
                UniformFieldData::Mat4x4f(matrix.projection.to_cols_array_2d())
            }
            CameraField::View => UniformFieldData::Mat4x4f(matrix.view.to_cols_array_2d()),
            CameraField::ProjectionView => {
                UniformFieldData::Mat4x4f(matrix.projection_view.to_cols_array_2d())
            }
            CameraField::InverseProjection => {
                UniformFieldData::Mat4x4f(matrix.inv_proj.to_cols_array_2d())
            }
            CameraField::InverseView => {
                UniformFieldData::Mat4x4f(matrix.inverse_view.to_cols_array_2d())
            }
        }
    }
}
