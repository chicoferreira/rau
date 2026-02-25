use slotmap::new_key_type;
use wgpu::util::DeviceExt;

use crate::project::Project;

new_key_type! {
    pub struct UniformId;
}

impl Project {
    pub fn get_uniform(&self, id: UniformId) -> Option<&Uniform> {
        self.uniforms.get(id)
    }

    pub fn get_uniform_mut(&mut self, id: UniformId) -> Option<&mut Uniform> {
        self.uniforms.get_mut(id)
    }

    pub fn list_uniforms(&self) -> impl Iterator<Item = (UniformId, &Uniform)> {
        self.uniforms.iter()
    }

    pub fn list_uniforms_mut(&mut self) -> impl Iterator<Item = (UniformId, &mut Uniform)> {
        self.uniforms.iter_mut()
    }

    pub fn is_empty_uniforms(&self) -> bool {
        self.uniforms.is_empty()
    }

    pub fn register_uniform(
        &mut self,
        device: &wgpu::Device,
        label: impl Into<String>,
        data: UniformData,
    ) -> UniformId {
        let label = label.into();

        let buffer = Uniform::create_buffer(device, &label, &data.cast());
        let uniform = Uniform {
            label,
            data,
            buffer,
        };

        self.uniforms.insert(uniform)
    }

    pub fn unregister_uniform(&mut self, id: UniformId) {
        self.uniforms.remove(id);
    }
}

#[derive(Debug)]
pub struct Uniform {
    pub label: String,
    pub data: UniformData,
    buffer: wgpu::Buffer,
}

impl Uniform {
    fn create_buffer(device: &wgpu::Device, label: &str, contents: &[u8]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let size = self.buffer.size();
        let content = self.data.cast();

        if size != content.len() as wgpu::BufferAddress {
            self.buffer = Self::create_buffer(device, &self.label, &content);
        } else {
            queue.write_buffer(&self.buffer, 0, &self.data.cast());
        }
    }

    pub fn set_and_upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        new_data: UniformData,
    ) {
        self.data = new_data;
        self.upload(device, queue);
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[derive(Debug, Default)]
pub struct UniformData {
    pub fields: Vec<UniformField>,
}

impl UniformData {
    pub fn cast(&self) -> Vec<u8> {
        let mut buf = vec![];
        let mut struct_align = 1;

        for field in &self.fields {
            let (align, _) = field.ty.layout();

            struct_align = std::cmp::max(struct_align, align);

            let new_len = buf.len().next_multiple_of(align);
            buf.resize(new_len, 0);

            field.ty.write_to(&mut buf);
        }

        let final_len = buf.len().next_multiple_of(struct_align);
        buf.resize(final_len, 0);

        buf
    }

    pub fn layout(&self) -> (usize, usize) {
        let mut size = 0usize;
        let mut struct_align = 1usize;

        for field in &self.fields {
            let (align, field_size) = field.ty.layout();
            struct_align = struct_align.max(align);
            size = size.next_multiple_of(align);
            size += field_size;
        }

        (size.next_multiple_of(struct_align), struct_align)
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

    pub fn new_from_kind(name: impl Into<String>, kind: UniformFieldKind) -> Self {
        match kind {
            UniformFieldKind::Vec4f => Self::new_vec4(name, [0.0; 4]),
            UniformFieldKind::Vec3f => Self::new_vec3(name, [0.0; 3]),
            UniformFieldKind::Vec2f => Self::new_vec2(name, [0.0; 2]),
            UniformFieldKind::Rgb => Self::new_rgb(name, [1.0; 3]),
            UniformFieldKind::Rgba => Self::new_rgba(name, [1.0; 4]),
            UniformFieldKind::Mat4x4f => Self::new_mat4(name, [[0.0; 4]; 4]),
        }
    }

    pub fn new_vec4(name: impl Into<String>, vec4: [f32; 4]) -> Self {
        Self::new(name, UniformFieldType::Vec4f(vec4))
    }

    pub fn new_vec3(name: impl Into<String>, vec3: [f32; 3]) -> Self {
        Self::new(name, UniformFieldType::Vec3f(vec3))
    }

    pub fn new_vec2(name: impl Into<String>, vec2: [f32; 2]) -> Self {
        Self::new(name, UniformFieldType::Vec2f(vec2))
    }

    pub fn new_rgb(name: impl Into<String>, color: [f32; 3]) -> Self {
        Self::new(name, UniformFieldType::Rgb(color))
    }

    pub fn new_rgba(name: impl Into<String>, color: [f32; 4]) -> Self {
        Self::new(name, UniformFieldType::Rgba(color))
    }

    pub fn new_mat4(name: impl Into<String>, mat4: [[f32; 4]; 4]) -> Self {
        Self::new(name, UniformFieldType::Mat4x4f(mat4))
    }
}

#[derive(Debug, Clone)]
pub enum UniformFieldType {
    Vec2f([f32; 2]),
    Vec3f([f32; 3]),
    Vec4f([f32; 4]),
    Rgb([f32; 3]),
    Rgba([f32; 4]),
    Mat4x4f([[f32; 4]; 4]),
}

impl UniformFieldType {
    // (alignment, size)
    pub fn layout(&self) -> (usize, usize) {
        match self {
            UniformFieldType::Vec2f(_) => (8, 8),
            UniformFieldType::Vec3f(_)
            | UniformFieldType::Rgb(_)
            | UniformFieldType::Vec4f(_)
            | UniformFieldType::Rgba(_) => (16, 16),
            UniformFieldType::Mat4x4f(_) => (16, 64),
        }
    }

    fn write_to(&self, buf: &mut Vec<u8>) {
        let start = buf.len();

        match self {
            UniformFieldType::Vec2f(v) => {
                buf.extend_from_slice(bytemuck::bytes_of(v));
            }
            UniformFieldType::Vec3f(v) | UniformFieldType::Rgb(v) => {
                let padded: [f32; 4] = [v[0], v[1], v[2], 0.0];
                buf.extend_from_slice(bytemuck::bytes_of(&padded));
            }
            UniformFieldType::Vec4f(v) | UniformFieldType::Rgba(v) => {
                buf.extend_from_slice(bytemuck::bytes_of(v));
            }
            UniformFieldType::Mat4x4f(m) => {
                buf.extend_from_slice(bytemuck::cast_slice(&m[..].concat()));
            }
        }

        let (_, size) = self.layout();
        debug_assert_eq!(buf.len(), start + size);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniformFieldKind {
    Vec2f,
    Vec3f,
    Vec4f,
    Rgb,
    Rgba,
    Mat4x4f,
}

impl UniformFieldKind {
    pub fn label(&self) -> &'static str {
        match self {
            UniformFieldKind::Vec2f => "Vec2f",
            UniformFieldKind::Vec3f => "Vec3f",
            UniformFieldKind::Vec4f => "Vec4f",
            UniformFieldKind::Rgb => "Rgb",
            UniformFieldKind::Rgba => "Rgba",
            UniformFieldKind::Mat4x4f => "Mat4x4f",
        }
    }

    pub fn wgsl_type_label(&self) -> &'static str {
        match self {
            UniformFieldKind::Vec2f => "vec2<f32>",
            UniformFieldKind::Vec3f => "vec3<f32>",
            UniformFieldKind::Vec4f => "vec4<f32>",
            UniformFieldKind::Rgb => "vec3<f32>",
            UniformFieldKind::Rgba => "vec4<f32>",
            UniformFieldKind::Mat4x4f => "mat4x4<f32>",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            UniformFieldKind::Vec2f,
            UniformFieldKind::Vec3f,
            UniformFieldKind::Vec4f,
            UniformFieldKind::Rgb,
            UniformFieldKind::Rgba,
            UniformFieldKind::Mat4x4f,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_pads_vec2_to_vec4_alignment() {
        let result = UniformData {
            fields: vec![
                UniformField::new_vec2("uv", [1.0, 2.0]),
                UniformField::new_vec4("tint", [3.0, 4.0, 5.0, 6.0]),
            ],
        }
        .cast();

        let result: &[f32] = bytemuck::cast_slice(&result);
        assert_eq!(result, &[1.0, 2.0, 0.0, 0.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn cast_pads_vec2_to_rgb_alignment() {
        let result = UniformData {
            fields: vec![
                UniformField::new_vec2("uv", [1.5, 2.5]),
                UniformField::new_rgb("color", [0.1, 0.2, 0.3]),
            ],
        }
        .cast();

        let result: &[f32] = bytemuck::cast_slice(&result);
        assert_eq!(result, &[1.5, 2.5, 0.0, 0.0, 0.1, 0.2, 0.3, 0.0]);
    }

    #[test]
    fn cast_no_padding_between_vec3_and_vec2() {
        let result = UniformData {
            fields: vec![
                UniformField::new_vec3("position", [9.0, 8.0, 7.0]),
                UniformField::new_vec2("scale", [0.25, 0.5]),
            ],
        }
        .cast();

        let result: &[f32] = bytemuck::cast_slice(&result);
        assert_eq!(result, &[9.0, 8.0, 7.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
    }
}
