use slotmap::new_key_type;

use crate::{
    camera::Camera,
    project::{
        bindgroup::BindGroup, shader::Shader, storage::Storage, viewport::Viewport,
        uniform::Uniform,
    },
};

pub mod bindgroup;
pub mod shader;
pub mod storage;
pub mod uniform;
pub mod viewport;

new_key_type! {
    pub struct UniformId;
    pub struct ShaderId;
    pub struct ViewportId;
    pub struct BindGroupId;
}

pub struct Project {
    pub shaders: Storage<ShaderId, Shader>,
    pub viewports: Storage<ViewportId, Viewport>,
    pub uniforms: Storage<UniformId, Uniform>,
    pub bind_groups: Storage<BindGroupId, BindGroup>,
    pub camera: Camera,
}

impl Project {
    pub fn new(camera: Camera) -> Self {
        Self {
            shaders: Storage::new(),
            viewports: Storage::new(),
            uniforms: Storage::new(),
            bind_groups: Storage::new(),
            camera,
        }
    }
}
