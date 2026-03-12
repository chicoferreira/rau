use wgpu::util::DeviceExt;

use crate::project::{CameraId, camera::Camera, storage::Storage};

// We need this struct to avoid borrow checker being mad
// when we are iterating over uniforms from a project
// and then we need the project to update it
pub struct UniformProjectContext<'a> {
    pub cameras: &'a Storage<CameraId, Camera>,
}

pub struct Uniform {
    pub label: String,
    pub data: UniformData,
    buffer: wgpu::Buffer,
}

#[derive(Debug, Default)]
pub struct UniformData {
    pub fields: Vec<UniformField>,
}

#[derive(Debug)]
pub struct UniformField {
    pub name: String,
    pub source: UniformFieldSource,
    pub last_data: UniformFieldData,
}

#[derive(Debug)]
pub enum UniformFieldSource {
    UserDefined(UniformFieldData),
    Camera(Option<CameraId>, CameraField),
}

#[derive(Debug, Clone, Copy, PartialEq, strum::Display)]
pub enum UniformFieldSourceKind {
    #[strum(to_string = "{0}")]
    UserDefined(UniformFieldKind),
    #[strum(to_string = "Camera {0}")]
    Camera(CameraField),
}

#[derive(Clone, Debug, PartialEq)]
pub enum UniformFieldData {
    Vec2f([f32; 2]),
    Vec3f([f32; 3]),
    Vec4f([f32; 4]),
    Rgb([f32; 3]),
    Rgba([f32; 4]),
    Mat4x4f([[f32; 4]; 4]),
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum UniformFieldKind {
    Vec2f,
    Vec3f,
    Vec4f,
    Rgb,
    Rgba,
    Mat4x4f,
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum CameraField {
    Position,
    Projection,
    View,
    #[strum(to_string = "Projection View")]
    ProjectionView,
    #[strum(to_string = "Inverse Projection")]
    InverseProjection,
    #[strum(to_string = "Inverse View")]
    InverseView,
}

impl Uniform {
    pub fn new(device: &wgpu::Device, label: impl Into<String>, data: UniformData) -> Uniform {
        let label = label.into();

        let buffer = Uniform::create_buffer(device, &label, &data.cast());
        Uniform {
            label,
            data,
            buffer,
        }
    }

    fn create_buffer(device: &wgpu::Device, label: &str, contents: &[u8]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn update(
        &mut self,
        context: UniformProjectContext<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut updated = false;
        for field in &mut self.data.fields {
            updated |= field.update(&context);
        }

        if updated {
            self.upload(device, queue);
        }
    }

    fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let size = self.buffer.size();
        let content = self.data.cast();

        if size != content.len() as wgpu::BufferAddress {
            self.buffer = Self::create_buffer(device, &self.label, &content);
        } else {
            queue.write_buffer(&self.buffer, 0, &content);
        }
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

impl UniformData {
    fn cast(&self) -> Vec<u8> {
        let mut buf = vec![];
        let mut struct_align = 1;

        for field in &self.fields {
            let (align, _) = field.kind().layout();

            struct_align = std::cmp::max(struct_align, align);

            let new_len = buf.len().next_multiple_of(align);
            buf.resize(new_len, 0);

            field.last_data.write_to(&mut buf);
        }

        let final_len = buf.len().next_multiple_of(struct_align);
        buf.resize(final_len, 0);

        buf
    }

    pub fn layout(&self) -> (usize, usize) {
        let mut size = 0usize;
        let mut struct_align = 1usize;

        for field in &self.fields {
            let (align, field_size) = field.kind().layout();
            struct_align = struct_align.max(align);
            size = size.next_multiple_of(align);
            size += field_size;
        }

        (size.next_multiple_of(struct_align), struct_align)
    }
}

impl UniformField {
    pub fn new_camera_sourced(
        name: impl Into<String>,
        camera_id: Option<CameraId>,
        field: CameraField,
    ) -> Self {
        let data = field.default_data();
        Self {
            name: name.into(),
            source: UniformFieldSource::Camera(camera_id, field),
            last_data: data,
        }
    }

    pub fn new_user_defined(name: impl Into<String>, value: UniformFieldData) -> Self {
        Self {
            name: name.into(),
            source: UniformFieldSource::UserDefined(value.clone()),
            last_data: value,
        }
    }

    // TODO: Remove this constructors
    #[allow(unused)]
    pub fn new_user_defined_vec2f(name: impl Into<String>, value: [f32; 2]) -> Self {
        Self::new_user_defined(name, UniformFieldData::Vec2f(value))
    }

    pub fn new_user_defined_vec3f(name: impl Into<String>, value: [f32; 3]) -> Self {
        Self::new_user_defined(name, UniformFieldData::Vec3f(value))
    }

    #[allow(unused)]
    pub fn new_user_defined_vec4f(name: impl Into<String>, value: [f32; 4]) -> Self {
        Self::new_user_defined(name, UniformFieldData::Vec4f(value))
    }

    pub fn new_user_defined_rgb(name: impl Into<String>, value: [f32; 3]) -> Self {
        Self::new_user_defined(name, UniformFieldData::Rgb(value))
    }

    #[allow(unused)]
    pub fn new_user_defined_rgba(name: impl Into<String>, value: [f32; 4]) -> Self {
        Self::new_user_defined(name, UniformFieldData::Rgba(value))
    }

    pub fn new_from_kind(name: impl Into<String>, kind: UniformFieldSourceKind) -> Self {
        match kind {
            UniformFieldSourceKind::UserDefined(kind) => {
                Self::new_user_defined(name, UniformFieldData::from_kind(kind))
            }
            UniformFieldSourceKind::Camera(kind) => Self::new_camera_sourced(name, None, kind),
        }
    }

    pub fn kind(&self) -> UniformFieldKind {
        self.last_data.kind()
    }

    fn update(&mut self, context: &UniformProjectContext<'_>) -> bool {
        let new_data = self.source.compute(context);
        let updated = self.last_data != new_data;
        self.last_data = new_data;
        updated
    }
}

impl UniformFieldSource {
    fn compute(&self, context: &UniformProjectContext<'_>) -> UniformFieldData {
        match self {
            UniformFieldSource::UserDefined(data) => data.clone(),
            UniformFieldSource::Camera(camera_id, source) => {
                if let Some(camera_id) = camera_id
                    && let Some(camera) = context.cameras.get(*camera_id)
                {
                    source.compute(camera)
                } else {
                    source.default_data()
                }
            }
        }
    }

    pub fn kind(&self) -> UniformFieldSourceKind {
        match self {
            UniformFieldSource::UserDefined(data) => {
                UniformFieldSourceKind::UserDefined(data.kind())
            }
            UniformFieldSource::Camera(_, source) => UniformFieldSourceKind::Camera(source.clone()),
        }
    }
}

impl UniformFieldData {
    fn from_kind(kind: UniformFieldKind) -> Self {
        match kind {
            UniformFieldKind::Vec2f => UniformFieldData::Vec2f([0.0; 2]),
            UniformFieldKind::Vec3f => UniformFieldData::Vec3f([0.0; 3]),
            UniformFieldKind::Vec4f => UniformFieldData::Vec4f([0.0; 4]),
            UniformFieldKind::Rgb => UniformFieldData::Rgb([1.0; 3]),
            UniformFieldKind::Rgba => UniformFieldData::Rgba([1.0; 4]),
            UniformFieldKind::Mat4x4f => UniformFieldData::Mat4x4f([[0.0; 4]; 4]),
        }
    }

    fn kind(&self) -> UniformFieldKind {
        match self {
            UniformFieldData::Vec2f(_) => UniformFieldKind::Vec2f,
            UniformFieldData::Vec3f(_) => UniformFieldKind::Vec3f,
            UniformFieldData::Vec4f(_) => UniformFieldKind::Vec4f,
            UniformFieldData::Rgb(_) => UniformFieldKind::Rgb,
            UniformFieldData::Rgba(_) => UniformFieldKind::Rgba,
            UniformFieldData::Mat4x4f(_) => UniformFieldKind::Mat4x4f,
        }
    }

    fn write_to(&self, buf: &mut Vec<u8>) {
        let start = buf.len();

        match self {
            UniformFieldData::Vec2f(v) => {
                buf.extend_from_slice(bytemuck::bytes_of(v));
            }
            UniformFieldData::Vec3f(v) | UniformFieldData::Rgb(v) => {
                let padded: [f32; 4] = [v[0], v[1], v[2], 0.0];
                buf.extend_from_slice(bytemuck::bytes_of(&padded));
            }
            UniformFieldData::Vec4f(v) | UniformFieldData::Rgba(v) => {
                buf.extend_from_slice(bytemuck::bytes_of(v));
            }
            UniformFieldData::Mat4x4f(m) => {
                buf.extend_from_slice(bytemuck::cast_slice(&m[..].concat()));
            }
        }

        let (_, size) = self.kind().layout();
        debug_assert_eq!(buf.len(), start + size);
    }
}

impl UniformFieldKind {
    pub fn layout(&self) -> (usize, usize) {
        match self {
            UniformFieldKind::Vec2f => (8, 8),
            UniformFieldKind::Vec3f
            | UniformFieldKind::Rgb
            | UniformFieldKind::Vec4f
            | UniformFieldKind::Rgba => (16, 16),
            UniformFieldKind::Mat4x4f => (16, 64),
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
}

impl CameraField {
    fn default_data(&self) -> UniformFieldData {
        match self {
            CameraField::Projection => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::Position => UniformFieldData::Vec4f([0.0; 4]),
            CameraField::View => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::ProjectionView => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::InverseProjection => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
            CameraField::InverseView => UniformFieldData::Mat4x4f([[1.0; 4]; 4]),
        }
    }

    fn compute(&self, camera: &Camera) -> UniformFieldData {
        match self {
            CameraField::Position => {
                UniformFieldData::Vec4f(camera.position().to_homogeneous().into())
            }
            CameraField::Projection => UniformFieldData::Mat4x4f(camera.matrix().projection.into()),
            CameraField::View => UniformFieldData::Mat4x4f(camera.matrix().view.into()),
            CameraField::ProjectionView => {
                UniformFieldData::Mat4x4f(camera.matrix().projection_view.into())
            }
            CameraField::InverseProjection => {
                UniformFieldData::Mat4x4f(camera.matrix().inverse_projection.into())
            }
            CameraField::InverseView => {
                UniformFieldData::Mat4x4f(camera.matrix().inverse_view.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_pads_vec2_to_vec4_alignment() {
        let result = UniformData {
            fields: vec![
                UniformField::new_user_defined_vec2f("uv", [1.0, 2.0]),
                UniformField::new_user_defined_vec4f("tint", [3.0, 4.0, 5.0, 6.0]),
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
                UniformField::new_user_defined_vec2f("uv", [1.5, 2.5]),
                UniformField::new_user_defined_rgb("color", [0.1, 0.2, 0.3]),
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
                UniformField::new_user_defined_vec3f("position", [9.0, 8.0, 7.0]),
                UniformField::new_user_defined_vec2f("scale", [0.25, 0.5]),
            ],
        }
        .cast();

        let result: &[f32] = bytemuck::cast_slice(&result);
        assert_eq!(result, &[9.0, 8.0, 7.0, 0.0, 0.25, 0.5, 0.0, 0.0]);
    }
}
