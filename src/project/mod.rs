use slotmap::SlotMap;

use crate::project::{
    bindgroup::{BindGroup, BindGroupId},
    shader::{Shader, ShaderId},
    texture::{TextureEntry, TextureId},
    uniform::{Uniform, UniformId},
};

pub mod bindgroup;
pub mod shader;
pub mod texture;
pub mod uniform;

#[derive(Default)]
pub struct Project {
    shaders: SlotMap<ShaderId, Shader>,
    textures: SlotMap<TextureId, TextureEntry>,
    uniforms: SlotMap<UniformId, Uniform>,
    bind_groups: SlotMap<BindGroupId, BindGroup>,
}
