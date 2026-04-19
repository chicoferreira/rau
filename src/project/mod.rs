use slotmap::new_key_type;

use crate::{
    error::AppError,
    project::{
        bindgroup::BindGroup,
        camera::Camera,
        dimension::Dimension,
        model::Model,
        render_schedule::RenderSchedule,
        render_pass::RenderPass,
        sampler::Sampler,
        shader::Shader,
        storage::{RuntimeStorage, Storage},
        texture::Texture,
        texture_view::TextureView,
        uniform::Uniform,
        viewport::Viewport,
    },
};

pub mod bindgroup;
pub mod camera;
pub mod dimension;
pub mod model;
pub mod render_schedule;
pub mod render_pass;
pub mod sampler;
pub mod shader;
pub mod storage;
pub mod sync;
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
    pub struct RenderPassId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderScheduleId;

#[derive(Default)]
pub struct Project {
    pub shaders: Storage<Shader>,
    pub viewports: Storage<Viewport>,
    pub uniforms: Storage<Uniform>,
    pub bind_groups: Storage<BindGroup>,
    pub textures: Storage<Texture>,
    pub texture_views: Storage<TextureView>,
    pub samplers: Storage<Sampler>,
    pub dimensions: Storage<Dimension>,
    pub cameras: Storage<Camera>,
    pub models: Storage<Model>,
    pub render_passes: Storage<RenderPass>,
    pub render_schedule: RenderSchedule,
}

#[derive(Default)]
pub struct RuntimeProject {
    pub shaders: RuntimeStorage<Shader>,
    pub uniforms: RuntimeStorage<Uniform>,
    pub bind_groups: RuntimeStorage<BindGroup>,
    pub textures: RuntimeStorage<Texture>,
    pub texture_views: RuntimeStorage<TextureView>,
    pub samplers: RuntimeStorage<Sampler>,
    pub dimensions: RuntimeStorage<Dimension>,
    pub cameras: RuntimeStorage<Camera>,
    pub models: RuntimeStorage<Model>,
    pub render_passes: RuntimeStorage<RenderPass>,
    pub render_schedule: sync::RuntimeCell<()>,
}

impl Project {
    pub fn label<'a>(&'a self, id: impl Into<ProjectResourceId>) -> Option<&'a str> {
        let label_err = match id.into() {
            ProjectResourceId::Shader(id) => self.shaders.get_label(id),
            ProjectResourceId::Viewport(id) => self.viewports.get_label(id),
            ProjectResourceId::Uniform(id) => self.uniforms.get_label(id),
            ProjectResourceId::BindGroup(id) => self.bind_groups.get_label(id),
            ProjectResourceId::Texture(id) => self.textures.get_label(id),
            ProjectResourceId::TextureView(id) => self.texture_views.get_label(id),
            ProjectResourceId::RenderPass(id) => self.render_passes.get_label(id),
            ProjectResourceId::Sampler(id) => self.samplers.get_label(id),
            ProjectResourceId::Dimension(id) => self.dimensions.get_label(id),
            ProjectResourceId::Camera(id) => self.cameras.get_label(id),
            ProjectResourceId::Model(id) => self.models.get_label(id),
            ProjectResourceId::RenderSchedule(_) => Ok("Render Schedule"),
        };

        label_err.ok()
    }
}

impl RuntimeProject {
    pub fn iter_errors(&self) -> impl Iterator<Item = (ProjectResourceId, &AppError)> {
        self.shaders
            .get_errors()
            .chain(self.uniforms.get_errors())
            .chain(self.bind_groups.get_errors())
            .chain(self.textures.get_errors())
            .chain(self.texture_views.get_errors())
            .chain(self.samplers.get_errors())
            .chain(self.dimensions.get_errors())
            .chain(self.cameras.get_errors())
            .chain(self.models.get_errors())
            .chain(self.render_passes.get_errors())
            .chain(self.render_schedule.get_error(RenderScheduleId))
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
    RenderPass(RenderPassId),
    RenderSchedule(RenderScheduleId),
}

pub trait ProjectResource {
    type Id: Into<ProjectResourceId> + Copy + Eq + std::hash::Hash + std::fmt::Debug;

    fn label(&self) -> &str;
}
