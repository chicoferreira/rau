use slotmap::new_key_type;

use crate::{
    camera::Camera,
    project::{
        bindgroup::BindGroup, dimension::Dimension, sampler::Sampler, shader::Shader,
        storage::Storage, texture::Texture, texture_view::TextureView, uniform::Uniform,
        viewport::Viewport,
    },
};

pub mod bindgroup;
pub mod dimension;
pub mod sampler;
pub mod shader;
pub mod storage;
pub mod texture;
pub mod texture_view;
pub mod uniform;
pub mod viewport;

new_key_type! {
    pub struct UniformId;
    pub struct ShaderId;
    pub struct ViewportId;
    pub struct BindGroupId;
    pub struct TextureId;
    pub struct TextureViewId;
    pub struct SamplerId;
    pub struct DimensionId;
}

pub struct Project {
    pub shaders: Storage<ShaderId, Shader>,
    pub viewports: Storage<ViewportId, Viewport>,
    pub uniforms: Storage<UniformId, Uniform>,
    pub bind_groups: Storage<BindGroupId, BindGroup>,
    pub textures: Storage<TextureId, Texture>,
    pub texture_views: Storage<TextureViewId, TextureView>,
    pub samplers: Storage<SamplerId, Sampler>,
    pub dimensions: Storage<DimensionId, Dimension>,
    pub camera: Camera,
}

impl Project {
    pub fn new(camera: Camera) -> Self {
        Self {
            shaders: Storage::new(),
            viewports: Storage::new(),
            uniforms: Storage::new(),
            bind_groups: Storage::new(),
            textures: Storage::new(),
            texture_views: Storage::new(),
            samplers: Storage::new(),
            dimensions: Storage::new(),
            camera,
        }
    }
}
