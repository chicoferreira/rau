use crate::{
    file_storage::FileStorage,
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, ModelId, Project, RenderPassId,
        ResourceId, ResourceKind, SamplerId, ShaderId, TextureId, TextureViewId, UniformId,
        ViewportId, paths::FilePath,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameState {
    pub target: RenameTarget,
    pub current_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenameTarget {
    CreateResource(ResourceKind),
    CreateFile(FilePath),
    File(FilePath),
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Viewport(ViewportId),
    UniformField(UniformId, usize),
    Shader(ShaderId),
    Camera(CameraId),
    Dimension(DimensionId),
    Sampler(SamplerId),
    Texture(TextureId),
    TextureView(TextureViewId),
    Model(ModelId),
    RenderPass(RenderPassId),
    RenderPipeline(RenderPassId, usize),
    ComputePass(ComputePassId),
}

impl RenameTarget {
    /// Returns the label to use as the starting point for a rename operation.
    pub fn get_rename_label<'a>(&'a self, project: &'a Project) -> Option<&'a str> {
        match self {
            RenameTarget::CreateFile(_) => Some(""),
            RenameTarget::File(file_path) => file_path.file_name(),
            RenameTarget::CreateResource(_) => Some(""),
            RenameTarget::BindGroup(id) => project.label(*id),
            RenameTarget::Viewport(id) => project.label(*id),
            RenameTarget::Shader(id) => project.label(*id),
            RenameTarget::Camera(id) => project.label(*id),
            RenameTarget::Dimension(id) => project.label(*id),
            RenameTarget::Sampler(id) => project.label(*id),
            RenameTarget::Texture(id) => project.label(*id),
            RenameTarget::TextureView(id) => project.label(*id),
            RenameTarget::Uniform(id) => project.label(*id),
            RenameTarget::Model(id) => project.label(*id),
            RenameTarget::RenderPass(id) => project.label(*id),
            RenameTarget::ComputePass(id) => project.label(*id),
            RenameTarget::UniformField(id, index) => project
                .uniforms
                .get(*id)
                .ok()
                .and_then(|uniform| uniform.get_field(*index))
                .map(|field| field.label()),
            RenameTarget::RenderPipeline(id, index) => project
                .render_passes
                .get(*id)
                .ok()
                .and_then(|render_pass| render_pass.pipelines.get(*index))
                .map(|pipeline| pipeline.label.as_str()),
        }
    }

    pub fn apply(self, new_name: String, project: &mut Project, file_storage: &mut FileStorage) {
        match self {
            RenameTarget::CreateResource(resource_kind) => {
                if !new_name.is_empty() {
                    project.register_with_label(resource_kind, new_name);
                }
            }
            RenameTarget::CreateFile(file_path) => {
                if !new_name.is_empty() {
                    file_storage.create_file_in_background(file_path, new_name);
                }
            }
            RenameTarget::File(file_path) => {
                if !new_name.is_empty() {
                    file_storage.rename_file_in_background(file_path, new_name);
                }
            }
            RenameTarget::Uniform(id) => {
                if let Ok(uniform) = project.uniforms.get_mut(id) {
                    uniform.label = new_name;
                }
            }
            RenameTarget::UniformField(id, index) => {
                if let Ok(uniform) = project.uniforms.get_mut(id) {
                    uniform.set_field_label(index, new_name);
                }
            }
            RenameTarget::BindGroup(id) => {
                if let Ok(bind_group) = project.bind_groups.get_mut(id) {
                    bind_group.set_label(new_name);
                }
            }
            RenameTarget::Viewport(id) => {
                if let Ok(viewport) = project.viewports.get_mut(id) {
                    viewport.label = new_name;
                }
            }
            RenameTarget::Shader(id) => {
                if let Ok(shader) = project.shaders.get_mut(id) {
                    shader.label = new_name;
                }
            }
            RenameTarget::Camera(id) => {
                if let Ok(camera) = project.cameras.get_mut(id) {
                    camera.label = new_name;
                }
            }
            RenameTarget::Dimension(id) => {
                if let Ok(dimension) = project.dimensions.get_mut(id) {
                    dimension.label = new_name;
                }
            }
            RenameTarget::Sampler(id) => {
                if let Ok(sampler) = project.samplers.get_mut(id) {
                    sampler.set_label(new_name);
                }
            }
            RenameTarget::Texture(id) => {
                if let Ok(texture) = project.textures.get_mut(id) {
                    texture.set_label(new_name);
                }
            }
            RenameTarget::TextureView(texture_view_id) => {
                if let Ok(texture_view) = project.texture_views.get_mut(texture_view_id) {
                    texture_view.set_label(new_name);
                }
            }
            RenameTarget::Model(model_id) => {
                if let Ok(model) = project.models.get_mut(model_id) {
                    model.set_label(new_name);
                }
            }
            RenameTarget::RenderPass(render_pass_id) => {
                if let Ok(render_pass) = project.render_passes.get_mut(render_pass_id) {
                    render_pass.set_label(new_name);
                }
            }
            RenameTarget::RenderPipeline(render_pass_id, index) => {
                if let Ok(render_pass) = project.render_passes.get_mut(render_pass_id) {
                    if let Some(pipeline) = render_pass.pipelines.get_mut(index) {
                        pipeline.set_label(new_name);
                    }
                }
            }
            RenameTarget::ComputePass(compute_pass_id) => {
                if let Ok(compute_pass) = project.compute_passes.get_mut(compute_pass_id) {
                    compute_pass.set_label(new_name);
                }
            }
        }
    }
}

impl From<ResourceId> for Option<RenameTarget> {
    fn from(id: ResourceId) -> Self {
        match id {
            ResourceId::Uniform(id) => Some(RenameTarget::Uniform(id)),
            ResourceId::BindGroup(id) => Some(RenameTarget::BindGroup(id)),
            ResourceId::Viewport(id) => Some(RenameTarget::Viewport(id)),
            ResourceId::Shader(id) => Some(RenameTarget::Shader(id)),
            ResourceId::Camera(id) => Some(RenameTarget::Camera(id)),
            ResourceId::Dimension(id) => Some(RenameTarget::Dimension(id)),
            ResourceId::Sampler(id) => Some(RenameTarget::Sampler(id)),
            ResourceId::Texture(id) => Some(RenameTarget::Texture(id)),
            ResourceId::TextureView(id) => Some(RenameTarget::TextureView(id)),
            ResourceId::Model(id) => Some(RenameTarget::Model(id)),
            ResourceId::RenderPass(id) => Some(RenameTarget::RenderPass(id)),
            ResourceId::ComputePass(id) => Some(RenameTarget::ComputePass(id)),
            ResourceId::FramePlan(_) => None,
        }
    }
}
