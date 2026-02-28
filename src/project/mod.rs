use slotmap::new_key_type;

use crate::{
    camera::Camera,
    project::{
        bindgroup::BindGroup, shader::Shader, storage::Storage, texture::TextureEntry,
        uniform::Uniform,
    },
};

pub mod bindgroup;
pub mod shader;
pub mod storage;
pub mod texture;
pub mod uniform;

new_key_type! {
    pub struct UniformId;
    pub struct ShaderId;
    pub struct TextureId;
    pub struct BindGroupId;
}

pub struct Project {
    pub shaders: Storage<ShaderId, Shader>,
    pub textures: Storage<TextureId, TextureEntry>,
    pub uniforms: Storage<UniformId, Uniform>,
    pub bind_groups: Storage<BindGroupId, BindGroup>,
    pub camera: Camera,
}

impl Project {
    pub fn new(camera: Camera) -> Self {
        Self {
            shaders: Storage::new(),
            textures: Storage::new(),
            uniforms: Storage::new(),
            bind_groups: Storage::new(),
            camera,
        }
    }
}
