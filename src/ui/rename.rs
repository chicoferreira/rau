use crate::project::{
    BindGroupId, CameraId, DimensionId, SamplerId, ShaderId, TextureViewId, UniformId, ViewportId,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameState {
    pub target: RenameTarget,
    pub current_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenameTarget {
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Viewport(ViewportId),
    UniformField(UniformId, usize),
    Shader(ShaderId),
    Camera(CameraId),
    Dimension(DimensionId),
    Sampler(SamplerId),
    TextureView(TextureViewId),
}
