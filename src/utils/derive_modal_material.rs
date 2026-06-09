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
        },
    },
    utils::{
        derive::{derive_texture, derive_texture_view, texture_label_from_path},
        texture_format::TextureFormat,
    },
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
                        let label = texture_label_from_path(path, || {
                            format!("{} {texture_type}", material.label())
                        });
                        let texture_id = derive_texture(project, &label, format, path.clone());
                        let texture_view_id = derive_texture_view(project, texture_id);
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
