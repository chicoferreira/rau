use crate::project;

#[derive(Debug)]
pub struct Uniform {
    pub(crate) label: String,
    pub(crate) data: UniformData,
    pub(crate) buffer: wgpu::Buffer,
}

impl Uniform {
    pub fn upload(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, &self.data.cast());
    }

    pub fn set_and_upload(&mut self, queue: &wgpu::Queue, new_data: UniformData) {
        self.data = new_data;
        self.upload(queue);
    }
}

#[derive(Debug)]
pub struct UniformData {
    pub fields: Vec<UniformField>,
}

impl UniformData {
    pub fn cast(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for field in &self.fields {
            // TODO: implement alignment solver
            data.extend_from_slice(field.ty.cast());
        }
        data
    }
}

#[derive(Debug)]
pub struct UniformField {
    pub name: String,
    pub ty: UniformFieldType,
}

impl UniformField {
    fn new(name: impl Into<String>, ty: UniformFieldType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }

    pub fn new_vec4(name: impl Into<String>, vec4: [f32; 4]) -> Self {
        Self::new(name, UniformFieldType::Vec4(vec4))
    }

    pub fn new_color(name: impl Into<String>, color: [f32; 4]) -> Self {
        Self::new(name, UniformFieldType::Color(color))
    }

    pub fn new_mat4(name: impl Into<String>, mat4: [[f32; 4]; 4]) -> Self {
        Self::new(name, UniformFieldType::Mat4(mat4))
    }
}

#[derive(Debug, Clone)]
pub enum UniformFieldType {
    // TODO: Support more types
    // https://sotrh.github.io/learn-wgpu/showcase/alignment/#alignment-of-vertex-and-index-buffers
    Vec4([f32; 4]),
    Color([f32; 4]),
    Mat4([[f32; 4]; 4]),
}

impl UniformFieldType {
    pub fn cast(&self) -> &[u8] {
        match self {
            UniformFieldType::Vec4(vec4) => bytemuck::cast_slice(vec4),
            UniformFieldType::Color(color) => bytemuck::cast_slice(color),
            UniformFieldType::Mat4(mat4) => bytemuck::cast_slice(mat4),
        }
    }
}

pub struct BindGroup {
    pub(crate) label: String,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) group: wgpu::BindGroup,
    pub(crate) entries: Vec<BindGroupEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupEntry {
    pub binding: u32,
    pub resource: BindGroupResource,
}

impl BindGroupEntry {
    pub fn into_bind_group_entry<'a>(
        &self,
        project: &'a project::Project,
    ) -> wgpu::BindGroupEntry<'a> {
        let resource = match self.resource {
            BindGroupResource::Texture { texture_id, .. } => {
                let texture = project
                    .get_texture(texture_id)
                    .expect("deal with this later");
                wgpu::BindingResource::TextureView(&texture.texture().view)
            }
            BindGroupResource::Sampler { texture_id, .. } => {
                let texture = project
                    .get_texture(texture_id)
                    .expect("deal with this later");
                wgpu::BindingResource::Sampler(&texture.texture().sampler)
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform = project
                    .get_uniform(uniform_id)
                    .expect("deal with this later");

                uniform.buffer.as_entire_binding()
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
        texture_id: project::TextureId,
        view_dimension: wgpu::TextureViewDimension,
    },
    Sampler {
        texture_id: project::TextureId,
        sampler_binding_type: wgpu::SamplerBindingType,
    },
    Uniform(project::UniformId),
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
