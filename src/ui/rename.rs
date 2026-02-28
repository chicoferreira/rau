use crate::project::{BindGroupId, ShaderId, TextureId, UniformId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameState {
    pub target: RenameTarget,
    pub current_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenameTarget {
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Viewport(TextureId),
    UniformField(UniformId, usize),
    Shader(ShaderId),
}
