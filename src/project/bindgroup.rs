use crate::project::{Project, TextureId, UniformId};

pub struct BindGroup {
    pub(crate) label: String,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) group: wgpu::BindGroup,
    pub(crate) entries: Vec<BindGroupEntry>,
}

impl BindGroup {
    pub fn new(
        project: &Project,
        device: &wgpu::Device,
        label: impl Into<String>,
        entries: Vec<BindGroupEntry>,
    ) -> BindGroup {
        let label = label.into();

        let layout_entries = entries.iter().copied().map(Into::into).collect::<Vec<_>>();

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });

        let group_entries = entries
            .iter()
            .map(|entry| entry.into_bind_group_entry(project))
            .collect::<Vec<_>>();

        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&label),
            layout: &layout,
            entries: &group_entries,
        });

        BindGroup {
            label,
            layout,
            group,
            entries,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupEntry {
    pub binding: u32,
    pub resource: BindGroupResource,
}

impl BindGroupEntry {
    pub fn into_bind_group_entry<'a>(&self, project: &'a Project) -> wgpu::BindGroupEntry<'a> {
        let resource = match self.resource {
            BindGroupResource::Texture { texture_id, .. } => {
                let texture = project
                    .textures
                    .get(texture_id)
                    .expect("deal with this later");
                wgpu::BindingResource::TextureView(&texture.texture.view)
            }
            BindGroupResource::Sampler { texture_id, .. } => {
                let texture = project
                    .textures
                    .get(texture_id)
                    .expect("deal with this later");
                wgpu::BindingResource::Sampler(&texture.texture.sampler)
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform = project
                    .uniforms
                    .get(uniform_id)
                    .expect("deal with this later");

                uniform.buffer().as_entire_binding()
            }
        };

        wgpu::BindGroupEntry {
            binding: self.binding,
            resource,
        }
    }
}

impl From<BindGroupEntry> for wgpu::BindGroupLayoutEntry {
    fn from(value: BindGroupEntry) -> Self {
        wgpu::BindGroupLayoutEntry {
            binding: value.binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: value.resource.into(),
            count: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindGroupResource {
    Texture {
        texture_id: TextureId,
        view_dimension: wgpu::TextureViewDimension,
    },
    Sampler {
        texture_id: TextureId,
        sampler_binding_type: wgpu::SamplerBindingType,
    },
    Uniform(UniformId),
}

impl From<BindGroupResource> for wgpu::BindingType {
    fn from(value: BindGroupResource) -> Self {
        match value {
            BindGroupResource::Texture { view_dimension, .. } => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                // TODO: support for depth texture
                view_dimension,
                multisampled: false,
            },
            BindGroupResource::Sampler {
                sampler_binding_type,
                ..
            } => wgpu::BindingType::Sampler(sampler_binding_type),
            BindGroupResource::Uniform(_) => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        }
    }
}
