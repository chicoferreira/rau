//! Builds one bind group per material of a model, creating the textures,
//! texture views and (optionally) the sampler the materials reference.

use std::collections::HashMap;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, Project, SamplerId, TextureViewId,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            model::{Material, TextureType},
            sampler::{Sampler, SamplerSpec},
            texture::{Texture, TextureSource},
            texture_view::TextureView,
        },
    },
    utils::texture_format::TextureFormat,
};

/// How the sampler binding (if any) is provided for the generated bind groups.
#[derive(Clone, PartialEq)]
pub enum SamplerSetting {
    /// Don't add a sampler binding.
    None,
    /// Create a new default sampler.
    CreateNew,
    /// Reuse an existing sampler.
    Existing(SamplerId),
}

/// Settings describing how to build the per-material bind groups.
pub struct MaterialBindGroupsConfig {
    /// Texture types to include, paired with the format each created texture
    /// uses. Every material is expected to reference a texture for each type.
    pub textures: Vec<(TextureType, TextureFormat)>,
    pub sampler: SamplerSetting,
}

impl MaterialBindGroupsConfig {
    /// Creates the textures, texture views, optional sampler and one bind group
    /// per material, returning the per-material bind group ids in material order.
    ///
    /// Textures are deduplicated by image path, so an image shared between
    /// materials only creates one texture and texture view.
    ///
    /// `model_label` is only used to name a newly created sampler. The caller is
    /// responsible for assigning the returned ids to the model.
    pub fn create_bind_groups(
        &self,
        project: &mut Project,
        materials: &[Material],
        model_label: &str,
    ) -> AppResult<Vec<Option<BindGroupId>>> {
        let sampler_id = match self.sampler {
            SamplerSetting::None => None,
            SamplerSetting::Existing(sampler_id) => {
                // Make sure the selected sampler still exists
                project.samplers.get(sampler_id)?;
                Some(sampler_id)
            }
            SamplerSetting::CreateNew => {
                let label = project
                    .samplers
                    .next_label(&format!("{model_label} Sampler"));
                Some(
                    project
                        .samplers
                        .register(Sampler::new(label, SamplerSpec::default())),
                )
            }
        };

        let mut texture_views_by_path: HashMap<FilePath, TextureViewId> = HashMap::new();
        let mut bind_group_ids = Vec::with_capacity(materials.len());

        for material in materials {
            let mut entries = Vec::new();

            for (texture_type, format) in self.textures.iter().copied() {
                let path = material.get_texture_path(texture_type).ok_or_else(|| {
                    AppError::uninit_field(format!(
                        "{texture_type} texture of material {:?}",
                        material.label()
                    ))
                })?;

                let texture_view_id = match texture_views_by_path.get(path) {
                    Some(texture_view_id) => *texture_view_id,
                    None => {
                        let texture_view_id =
                            create_texture_and_view(project, material, texture_type, format, path);
                        texture_views_by_path.insert(path.clone(), texture_view_id);
                        texture_view_id
                    }
                };

                entries.push(BindGroupEntry::new_vertex_fragment(
                    BindGroupResource::Texture {
                        texture_view_id: Some(texture_view_id),
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                ));
            }

            if let Some(sampler_id) = sampler_id {
                entries.push(BindGroupEntry::new_vertex_fragment(
                    BindGroupResource::Sampler {
                        sampler_id: Some(sampler_id),
                        sampler_binding_type: wgpu::SamplerBindingType::Filtering,
                    },
                ));
            }

            let label = project
                .bind_groups
                .next_label(&format!("{model_label} {} Bind Group", material.label()));
            let bind_group_id = project.bind_groups.register(BindGroup::new(label, entries));
            bind_group_ids.push(Some(bind_group_id));
        }

        Ok(bind_group_ids)
    }
}

fn create_texture_and_view(
    project: &mut Project,
    material: &Material,
    texture_type: TextureType,
    format: TextureFormat,
    path: &FilePath,
) -> TextureViewId {
    let texture_label = project
        .textures
        .next_label(&texture_label(path, material, texture_type));

    let texture = Texture::new(
        texture_label.clone(),
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        TextureSource::Image(Some(path.clone())),
    );
    let texture_id = project.textures.register(texture);

    let texture_view_label = project
        .texture_views
        .next_label(&format!("{texture_label} View"));
    project.texture_views.register(TextureView::new(
        texture_view_label,
        Some(texture_id),
        None,
        None,
    ))
}

fn texture_label(path: &FilePath, material: &Material, texture_type: TextureType) -> String {
    path.file_stem()
        .filter(|stem| !stem.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{} {texture_type}", material.label()))
}

/// Color textures get an srgb format, while data textures (normals, etc.)
/// get a linear one.
pub fn default_texture_format(texture_type: TextureType) -> TextureFormat {
    match texture_type {
        TextureType::Ambient | TextureType::Diffuse | TextureType::Specular => {
            TextureFormat::Rgba8UnormSrgb
        }
        TextureType::Normal | TextureType::Shininess | TextureType::Dissolve => {
            TextureFormat::Rgba8Unorm
        }
    }
}
