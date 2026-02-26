use slotmap::SlotMap;

use crate::{
    camera::Camera,
    project::{
        bindgroup::{BindGroup, BindGroupId},
        shader::{Shader, ShaderId},
        texture::{TextureEntry, TextureId},
        uniform::{Uniform, UniformId},
    },
};

pub mod bindgroup;
pub mod shader;
pub mod texture;
pub mod uniform;

pub struct Project {
    shaders: SlotMap<ShaderId, Shader>,
    textures: SlotMap<TextureId, TextureEntry>,
    uniforms: SlotMap<UniformId, Uniform>,
    bind_groups: SlotMap<BindGroupId, BindGroup>,
    camera: Camera,
}

impl Project {
    pub fn new(camera: Camera) -> Self {
        Self {
            shaders: Default::default(),
            textures: Default::default(),
            uniforms: Default::default(),
            bind_groups: Default::default(),
            camera,
        }
    }

    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}
