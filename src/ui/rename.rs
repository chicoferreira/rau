use crate::project::{BindGroupId, CameraId, ShaderId, UniformId, ViewportId};

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
}
