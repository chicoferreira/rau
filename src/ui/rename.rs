use crate::project::{bindgroup::BindGroupId, texture::TextureId, uniform::UniformId};

pub struct RenameState {
    pub target: RenameTarget,
    pub current_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameTarget {
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Viewport(TextureId),
}
