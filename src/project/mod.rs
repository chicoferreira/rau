use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

use crate::{
    error::{AppError, AppResult},
    project::{
        render::PresentationRender,
        resource::{
            bindgroup::BindGroup, camera::Camera, compute_pass::ComputePass, dimension::Dimension,
            model::Model, presentation::Presentation, render_pass::RenderPass,
            render_pipeline::RenderPipeline, sampler::Sampler, shader::Shader, texture::Texture,
            texture_view::TextureView, uniform::Uniform, viewport::Viewport,
        },
        storage::{RuntimeStorage, Storage},
        sync::Revision,
    },
};

/// A snapshot of every resource's project revision, used to detect when any
/// resource has changed since a given point in time.
pub type ProjectRevisionSnapshot = Vec<(ResourceId, Revision)>;

pub mod macros;
pub mod paths;
pub mod render;
pub mod resource;
pub mod save;
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
    pub struct RenderPipelineId;
    pub struct ComputePassId;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PresentationId;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub presentation: Presentation,
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
    pub render_pipelines: Storage<RenderPipeline>,
    pub render_passes: Storage<RenderPass>,
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
    pub render_pipelines: RuntimeStorage<RenderPipeline>,
    pub compute_passes: RuntimeStorage<ComputePass>,
    pub presentation_render: PresentationRender,
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
            ResourceId::RenderPipeline(id) => self.render_pipelines.get_label(id),
            ResourceId::RenderPass(id) => self.render_passes.get_label(id),
            ResourceId::Sampler(id) => self.samplers.get_label(id),
            ResourceId::Dimension(id) => self.dimensions.get_label(id),
            ResourceId::Camera(id) => self.cameras.get_label(id),
            ResourceId::Model(id) => self.models.get_label(id),
            ResourceId::Presentation(_) => Ok("Presentation"),
            ResourceId::ComputePass(id) => self.compute_passes.get_label(id),
        };

        label_err.ok()
    }

    pub fn register_with_label(&mut self, kind: ResourceKind, label: String) -> Option<ResourceId> {
        let id = match kind {
            ResourceKind::Shader => self.shaders.create(label).into(),
            ResourceKind::Viewport => self.viewports.create(label).into(),
            ResourceKind::Uniform => self.uniforms.create(label).into(),
            ResourceKind::BindGroup => self.bind_groups.create(label).into(),
            ResourceKind::Texture => self.textures.create(label).into(),
            ResourceKind::TextureView => self.texture_views.create(label).into(),
            ResourceKind::Sampler => self.samplers.create(label).into(),
            ResourceKind::Dimension => self.dimensions.create(label).into(),
            ResourceKind::Camera => self.cameras.create(label).into(),
            ResourceKind::Model => self.models.create(label).into(),
            ResourceKind::RenderPipeline => self.render_pipelines.create(label).into(),
            ResourceKind::RenderPass => self.render_passes.create(label).into(),
            ResourceKind::Presentation => return None,
            ResourceKind::ComputePass => self.compute_passes.create(label).into(),
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
            ResourceId::Presentation(_) => {}
            ResourceId::ComputePass(id) => self.compute_passes.unregister(id),
            ResourceId::Viewport(id) => self.viewports.unregister(id),
            ResourceId::RenderPipeline(id) => self.render_pipelines.unregister(id),
            ResourceId::RenderPass(id) => self.render_passes.unregister(id),
        };
    }

    pub fn project_revisions(&self) -> impl Iterator<Item = (ResourceId, Revision)> {
        self.shaders
            .project_revisions()
            .chain(self.viewports.project_revisions())
            .chain(self.uniforms.project_revisions())
            .chain(self.bind_groups.project_revisions())
            .chain(self.textures.project_revisions())
            .chain(self.texture_views.project_revisions())
            .chain(self.samplers.project_revisions())
            .chain(self.dimensions.project_revisions())
            .chain(self.cameras.project_revisions())
            .chain(self.models.project_revisions())
            .chain(self.render_pipelines.project_revisions())
            .chain(self.render_passes.project_revisions())
            .chain(self.compute_passes.project_revisions())
            .chain(std::iter::once((
                ResourceId::Presentation(PresentationId),
                self.presentation.project_revision(),
            )))
    }

    pub fn snapshot(&self) -> ProjectRevisionSnapshot {
        self.project_revisions().collect()
    }

    pub fn serialize(&self) -> AppResult<Vec<u8>> {
        serde_json::to_vec(&self).map_err(Into::into)
    }

    pub fn deserialize(data: &[u8]) -> AppResult<Self> {
        serde_json::from_slice(data).map_err(Into::into)
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
            ResourceId::RenderPipeline(id) => self.render_pipelines.unregister(id),
            ResourceId::RenderPass(_) => {}
            ResourceId::Presentation(_) => {}
            ResourceId::ComputePass(id) => self.compute_passes.unregister(id),
            ResourceId::Viewport(_) => {}
        };
    }

    pub fn is_rebuilding(&self) -> bool {
        self.shaders.has_pending()
            || self.uniforms.has_pending()
            || self.bind_groups.has_pending()
            || self.textures.has_pending()
            || self.texture_views.has_pending()
            || self.samplers.has_pending()
            || self.dimensions.has_pending()
            || self.cameras.has_pending()
            || self.models.has_pending()
            || self.render_pipelines.has_pending()
            || self.compute_passes.has_pending()
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
            .chain(self.render_pipelines.get_errors())
            .chain(self.compute_passes.get_errors())
            .chain(
                self.presentation_render
                    .error()
                    .map(|error| (ResourceId::Presentation(PresentationId), error)),
            )
    }

    pub fn get_error(&self, id: impl Into<ResourceId>) -> Option<&AppError> {
        match id.into() {
            ResourceId::Shader(id) => self.shaders.get_error(id),
            ResourceId::Uniform(id) => self.uniforms.get_error(id),
            ResourceId::BindGroup(id) => self.bind_groups.get_error(id),
            ResourceId::Texture(id) => self.textures.get_error(id),
            ResourceId::TextureView(id) => self.texture_views.get_error(id),
            ResourceId::Sampler(id) => self.samplers.get_error(id),
            ResourceId::Dimension(id) => self.dimensions.get_error(id),
            ResourceId::Camera(id) => self.cameras.get_error(id),
            ResourceId::Model(id) => self.models.get_error(id),
            ResourceId::RenderPipeline(id) => self.render_pipelines.get_error(id),
            ResourceId::ComputePass(id) => self.compute_passes.get_error(id),
            ResourceId::Presentation(_) => self.presentation_render.error(),
            ResourceId::Viewport(_) | ResourceId::RenderPass(_) => None,
        }
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
    RenderPipeline(RenderPipelineId),
    RenderPass(RenderPassId),
    Presentation(PresentationId),
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
    RenderPipeline,
    RenderPass,
    Presentation,
    ComputePass,
}

pub trait ProjectResource {
    type Id: Into<ResourceId> + Copy + Eq + std::hash::Hash + std::fmt::Debug + Send + Sync;

    fn label(&self) -> &str;

    fn project_revision(&self) -> Revision;
}

pub trait Creatable: ProjectResource {
    fn create(label: String) -> Self;
}
