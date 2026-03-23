use crate::project::{
    BindGroupId, CameraId, DimensionId, Project, SamplerId, ShaderId, TextureId, TextureViewId,
    UniformId, ViewportId,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RenameState {
    pub target: RenameTarget,
    pub current_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenameTarget {
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
}

impl RenameTarget {
    pub fn get_label<'a>(&self, project: &'a Project) -> Option<&'a str> {
        match *self {
            RenameTarget::BindGroup(id) => project.label(id),
            RenameTarget::Viewport(id) => project.label(id),
            RenameTarget::Shader(id) => project.label(id),
            RenameTarget::Camera(id) => project.label(id),
            RenameTarget::Dimension(id) => project.label(id),
            RenameTarget::Sampler(id) => project.label(id),
            RenameTarget::Texture(id) => project.label(id),
            RenameTarget::TextureView(id) => project.label(id),
            RenameTarget::Uniform(id) => project.label(id),
            RenameTarget::UniformField(id, index) => project
                .uniforms
                .get(id)
                .ok()
                .and_then(|uniform| uniform.get_field(index))
                .map(|field| field.label()),
        }
    }

    pub fn apply(self, new_name: String, project: &mut Project) {
        match self {
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
                    bind_group.label = new_name;
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
                    texture.label = new_name;
                }
            }
            RenameTarget::TextureView(texture_view_id) => {
                if let Ok(texture_view) = project.texture_views.get_mut(texture_view_id) {
                    texture_view.set_label(new_name);
                }
            }
        }
    }
}
