use slotmap::new_key_type;

use crate::project::{
    bindgroup::BindGroup, camera::Camera, dimension::Dimension, model::Model, sampler::Sampler,
    shader::Shader, storage::Storage, texture::Texture, texture_view::TextureView,
    uniform::Uniform, viewport::Viewport,
};

pub mod bindgroup;
pub mod camera;
pub mod dimension;
pub mod model;
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
    pub struct ModelId;
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
    pub models: Storage<ModelId, Model>,
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
            models: Storage::new(),
        }
    }

    pub fn label<'a>(&'a self, id: impl Into<ProjectResourceId>) -> Option<&'a str> {
        let label_err = match id.into() {
            ProjectResourceId::Shader(id) => self.shaders.get_label(id),
            ProjectResourceId::Viewport(id) => self.viewports.get_label(id),
            ProjectResourceId::Uniform(id) => self.uniforms.get_label(id),
            ProjectResourceId::BindGroup(id) => self.bind_groups.get_label(id),
            ProjectResourceId::Texture(id) => self.textures.get_label(id),
            ProjectResourceId::TextureView(id) => self.texture_views.get_label(id),
            ProjectResourceId::Sampler(id) => self.samplers.get_label(id),
            ProjectResourceId::Dimension(id) => self.dimensions.get_label(id),
            ProjectResourceId::Camera(id) => self.cameras.get_label(id),
            ProjectResourceId::Model(id) => self.models.get_label(id),
        };

        label_err.ok()
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
    Model(ModelId),
}

pub trait ProjectResource {
    fn label(&self) -> &str;
}
