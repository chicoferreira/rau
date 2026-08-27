use crate::{
    file::file_storage::FileStorage,
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, ModelId, Project, RenderPassId,
        RenderPipelineId, ResourceId, ResourceKind, SamplerId, ShaderId, TextureId, TextureViewId,
        UniformId, ViewportId, paths::FilePath,
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
    CreateFolder(FilePath),
    FileOrFolder(FilePath),
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
    RenderPipeline(RenderPipelineId),
    ComputePass(ComputePassId),
}

impl RenameTarget {
    /// Returns the label to use as the starting point for a rename operation.
    pub fn get_rename_label<'a>(&'a self, project: &'a Project) -> Option<&'a str> {
        match self {
            RenameTarget::CreateFile(_) => Some(""),
            RenameTarget::CreateFolder(_) => Some(""),
            RenameTarget::FileOrFolder(file_path) => file_path.file_name(),
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
            RenameTarget::RenderPipeline(id) => project.label(*id),
            RenameTarget::RenderPass(id) => project.label(*id),
            RenameTarget::ComputePass(id) => project.label(*id),
            RenameTarget::UniformField(id, index) => project
                .uniforms
                .get(*id)
                .ok()
                .and_then(|uniform| uniform.get_field(*index))
                .map(|field| field.label()),
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
                    let file_path = match file_path.join(new_name) {
                        Ok(path) => path,
                        Err(e) => {
                            log::error!("Failed to join path: {}", e);
                            return;
                        }
                    };
                    file_storage.create_file_in_background(file_path);
                }
            }
            RenameTarget::CreateFolder(file_path) => {
                if !new_name.is_empty() {
                    let folder_path = match file_path.join(new_name) {
                        Ok(path) => path,
                        Err(e) => {
                            log::error!("Failed to join path: {}", e);
                            return;
                        }
                    };
                    file_storage.create_folder_in_background(folder_path);
                }
            }
            RenameTarget::FileOrFolder(file_path) => {
                if !new_name.is_empty() {
                    if file_path.file_name() == Some(new_name.as_str()) {
                        return;
                    }

                    let Some(parent_path) = file_path.parent() else {
                        return;
                    };

                    let new_path = match parent_path.join(new_name) {
                        Ok(path) => path,
                        Err(e) => {
                            log::error!("Failed to join path: {}", e);
                            return;
                        }
                    };

                    file_storage.move_path_in_background(file_path, new_path);
                }
            }
            RenameTarget::Uniform(id) => project.set_label(id, new_name),
            RenameTarget::UniformField(id, index) => {
                if let Ok(uniform) = project.uniforms.get_mut(id) {
                    uniform.set_field_label(index, new_name);
                }
            }
            RenameTarget::BindGroup(id) => project.set_label(id, new_name),
            RenameTarget::Viewport(id) => project.set_label(id, new_name),
            RenameTarget::Shader(id) => project.set_label(id, new_name),
            RenameTarget::Camera(id) => project.set_label(id, new_name),
            RenameTarget::Dimension(id) => project.set_label(id, new_name),
            RenameTarget::Sampler(id) => project.set_label(id, new_name),
            RenameTarget::Texture(id) => project.set_label(id, new_name),
            RenameTarget::TextureView(id) => project.set_label(id, new_name),
            RenameTarget::Model(id) => project.set_label(id, new_name),
            RenameTarget::RenderPass(id) => project.set_label(id, new_name),
            RenameTarget::RenderPipeline(id) => project.set_label(id, new_name),
            RenameTarget::ComputePass(id) => project.set_label(id, new_name),
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
            ResourceId::RenderPipeline(id) => Some(RenameTarget::RenderPipeline(id)),
            ResourceId::RenderPass(id) => Some(RenameTarget::RenderPass(id)),
            ResourceId::ComputePass(id) => Some(RenameTarget::ComputePass(id)),
            ResourceId::Presentation(_) => None,
        }
    }
}
