use slotmap::new_key_type;

use crate::project::{
    bindgroup::BindGroup, camera::Camera, dimension::Dimension, sampler::Sampler, shader::Shader,
    storage::Storage, texture::Texture, texture_view::TextureView, uniform::Uniform,
    viewport::Viewport,
};

pub mod bindgroup;
pub mod camera;
pub mod dimension;
pub mod recreate;
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
    pub struct CameraId;
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
    pub cameras: Storage<CameraId, Camera>,
}

impl Project {
    pub fn new() -> Self {
        Self {
            shaders: Storage::new(),
            viewports: Storage::new(),
            uniforms: Storage::new(),
            bind_groups: Storage::new(),
            textures: Storage::new(),
            texture_views: Storage::new(),
            samplers: Storage::new(),
            dimensions: Storage::new(),
            cameras: Storage::new(),
        }
    }

    pub fn label<'a>(&'a self, id: impl Into<ProjectResourceId>) -> Option<&'a str> {
        let id = id.into();
        match id {
            ProjectResourceId::Shader(id) => self.shaders.get(id).ok().map(|s| s.label.as_str()),
            ProjectResourceId::Viewport(id) => {
                self.viewports.get(id).ok().map(|v| v.label.as_str())
            }
            ProjectResourceId::Uniform(id) => self.uniforms.get(id).ok().map(|u| u.label.as_str()),
            ProjectResourceId::BindGroup(id) => {
                self.bind_groups.get(id).ok().map(|b| b.label.as_str())
            }
            ProjectResourceId::Texture(id) => self.textures.get(id).ok().map(|t| t.label.as_str()),
            ProjectResourceId::TextureView(id) => {
                self.texture_views.get(id).ok().map(|v| v.label())
            }
            ProjectResourceId::Sampler(id) => self.samplers.get(id).ok().map(|s| s.label()),
            ProjectResourceId::Dimension(id) => {
                self.dimensions.get(id).ok().map(|d| d.label.as_str())
            }
            ProjectResourceId::Camera(id) => self.cameras.get(id).ok().map(|c| c.label.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::From)]
pub enum ProjectResourceId {
    Shader(ShaderId),
    Viewport(ViewportId),
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Texture(TextureId),
    TextureView(TextureViewId),
    Sampler(SamplerId),
    Dimension(DimensionId),
    Camera(CameraId),
}
