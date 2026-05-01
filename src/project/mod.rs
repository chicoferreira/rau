use slotmap::new_key_type;

use crate::{
    error::AppError,
    project::resource::{
        bindgroup::BindGroup, camera::Camera, compute_pass::ComputePass, dimension::Dimension,
        frame_plan::FramePlan, model::Model, render_pass::RenderPass, sampler::Sampler,
        shader::Shader, texture::Texture, texture_view::TextureView, uniform::Uniform,
        viewport::Viewport,
    },
    project::storage::{RuntimeStorage, Storage},
};

pub mod file;
pub mod resource;
pub mod storage;
pub mod sync;

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
    pub struct ComputePassId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FramePlanId;

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
    pub frame_plan: FramePlan,
    pub compute_passes: Storage<ComputePass>,
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
    pub frame_plan: sync::RuntimeCell<(), <FramePlan as sync::SyncResource>::Job>,
    pub compute_passes: RuntimeStorage<ComputePass>,
}

impl Project {
    pub fn label<'a>(&'a self, id: impl Into<ResourceId>) -> Option<&'a str> {
        let label_err = match id.into() {
            ResourceId::Shader(id) => self.shaders.get_label(id),
            ResourceId::Viewport(id) => self.viewports.get_label(id),
            ResourceId::Uniform(id) => self.uniforms.get_label(id),
            ResourceId::BindGroup(id) => self.bind_groups.get_label(id),
            ResourceId::Texture(id) => self.textures.get_label(id),
            ResourceId::TextureView(id) => self.texture_views.get_label(id),
            ResourceId::RenderPass(id) => self.render_passes.get_label(id),
            ResourceId::Sampler(id) => self.samplers.get_label(id),
            ResourceId::Dimension(id) => self.dimensions.get_label(id),
            ResourceId::Camera(id) => self.cameras.get_label(id),
            ResourceId::Model(id) => self.models.get_label(id),
            ResourceId::FramePlan(_) => Ok("Frame Plan"),
            ResourceId::ComputePass(id) => self.compute_passes.get_label(id),
        };

        label_err.ok()
    }

    pub fn register(&mut self, kind: ResourceKind) -> Option<ResourceId> {
        let id = match kind {
            ResourceKind::Shader => todo!("not yet implemented"),
            ResourceKind::Viewport => self.viewports.create().into(),
            ResourceKind::Uniform => self.uniforms.create().into(),
            ResourceKind::BindGroup => self.bind_groups.create().into(),
            ResourceKind::Texture => todo!("not yet implemented"),
            ResourceKind::TextureView => self.texture_views.create().into(),
            ResourceKind::Sampler => self.samplers.create().into(),
            ResourceKind::Dimension => self.dimensions.create().into(),
            ResourceKind::Camera => self.cameras.create().into(),
            ResourceKind::Model => todo!("not yet implemented"),
            ResourceKind::RenderPass => self.render_passes.create().into(),
            ResourceKind::FramePlan => return None,
            ResourceKind::ComputePass => self.compute_passes.create().into(),
        };
        Some(id)
    }

    pub fn unregister(&mut self, id: ResourceId) {
        match id {
            ResourceId::Shader(id) => self.shaders.unregister(id),
            ResourceId::Uniform(id) => self.uniforms.unregister(id),
            ResourceId::BindGroup(id) => self.bind_groups.unregister(id),
            ResourceId::Texture(id) => self.textures.unregister(id),
            ResourceId::TextureView(id) => self.texture_views.unregister(id),
            ResourceId::Sampler(id) => self.samplers.unregister(id),
            ResourceId::Dimension(id) => self.dimensions.unregister(id),
            ResourceId::Camera(id) => self.cameras.unregister(id),
            ResourceId::Model(id) => self.models.unregister(id),
            ResourceId::FramePlan(_) => {}
            ResourceId::ComputePass(id) => self.compute_passes.unregister(id),
            ResourceId::Viewport(id) => self.viewports.unregister(id),
            ResourceId::RenderPass(id) => self.render_passes.unregister(id),
        };
    }
}

impl RuntimeProject {
    pub fn unregister(&mut self, id: ResourceId) {
        match id {
            ResourceId::Shader(id) => self.shaders.unregister(id),
            ResourceId::Uniform(id) => self.uniforms.unregister(id),
            ResourceId::BindGroup(id) => self.bind_groups.unregister(id),
            ResourceId::Texture(id) => self.textures.unregister(id),
            ResourceId::TextureView(id) => self.texture_views.unregister(id),
            ResourceId::Sampler(id) => self.samplers.unregister(id),
            ResourceId::Dimension(id) => self.dimensions.unregister(id),
            ResourceId::Camera(id) => self.cameras.unregister(id),
            ResourceId::Model(id) => self.models.unregister(id),
            ResourceId::RenderPass(id) => self.render_passes.unregister(id),
            ResourceId::FramePlan(_) => self.frame_plan = sync::RuntimeCell::Empty,
            ResourceId::ComputePass(id) => self.compute_passes.unregister(id),
            ResourceId::Viewport(_) => {}
        };
    }

    pub fn iter_errors(&self) -> impl Iterator<Item = (ResourceId, &AppError)> {
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
            .chain(self.compute_passes.get_errors())
            .chain(self.frame_plan.get_error(FramePlanId))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::From)]
pub enum ResourceId {
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
    FramePlan(FramePlanId),
    ComputePass(ComputePassId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceKind {
    Shader,
    Viewport,
    Uniform,
    BindGroup,
    Texture,
    TextureView,
    Sampler,
    Dimension,
    Camera,
    Model,
    RenderPass,
    FramePlan,
    ComputePass,
}

pub trait ProjectResource {
    type Id: Into<ResourceId> + Copy + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync;

    fn label(&self) -> &str;
}

pub trait Creatable: ProjectResource {
    const DEFAULT_LABEL: &'static str;

    fn create(label: String) -> Self;
}
