use egui_dnd::utils::shift_vec;

pub mod camera;
#[cfg(test)]
mod tests;

use crate::{
    error::AppResult,
    project::{
        CameraId, ProjectResource, UniformId, camera::Camera, recreate::{ProjectEvent, Recreatable, RecreateTracker}, storage::Storage, uniform::camera::CameraField
    },
    utils::resizable_buffer::{ChangeResult, ResizableBuffer},
};

pub struct UniformCreationContext<'a> {
    pub cameras: &'a Storage<CameraId, Camera>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct Uniform {
    pub label: String,
    fields: Vec<UniformField>,
    buffer: ResizableBuffer,
    dirty: bool,
}

type UniformFieldId = usize;

#[derive(Debug)]
pub struct UniformField {
    label: String,
    id: UniformFieldId, // Used for stability in reordering
    source: UniformFieldSource,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub enum UniformFieldSource {
    UserDefined(UniformFieldData),
    Camera {
        camera_id: Option<CameraId>,
        field: CameraField,
        current_value: UniformFieldData,
    },
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
pub enum UniformFieldDataKind {
    Vec2f,
    Vec3f,
    Vec4f,
    Rgb,
    Rgba,
    Mat4x4f,
}

impl Uniform {
    pub fn new(
        device: &wgpu::Device,
        label: impl Into<String>,
        fields: Vec<UniformField>,
    ) -> AppResult<Uniform> {
        let label = label.into();
        let content = cast_fields(&fields);
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        let buffer = ResizableBuffer::new(device, &label, usage, &content)?;
        Ok(Uniform {
            label,
            fields,
            buffer,
            dirty: false,
        })
    }

    pub fn buffer(&self) -> &ResizableBuffer {
        &self.buffer
    }

    pub fn fields(&self) -> &[UniformField] {
        &self.fields
    }

    pub fn get_field(&self, index: usize) -> Option<&UniformField> {
        self.fields.get(index)
    }

    pub fn add_field(&mut self, field: UniformField) {
        self.fields.push(field);
        self.dirty = true;
    }

    pub fn remove_field(&mut self, index: usize) {
        if index < self.fields.len() {
            self.fields.remove(index);
            self.dirty = true;
        }
    }

    pub fn set_field_label(&mut self, index: usize, new_name: String) {
        if index < self.fields.len() {
            self.fields[index].label = new_name;
            // changing the label does not affect the buffer
        }
    }

    pub fn set_field_source(&mut self, index: usize, source: UniformFieldSource) {
        if let Some(field) = self.fields.get_mut(index) {
            field.source = source;
            field.dirty = true;
            self.dirty = true;
        }
    }

    pub fn reorder_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.fields);
        self.dirty = true;
    }

    pub fn layout(&self) -> (usize, usize) {
        let mut size = 0usize;
        let mut struct_align = 1usize;

        for field in &self.fields {
            let (align, field_size) = field.source.get_value().kind().layout();
            struct_align = struct_align.max(align);
            size = size.next_multiple_of(align);
            size += field_size;
        }

        (size.next_multiple_of(struct_align), struct_align)
    }
}

impl ProjectResource for Uniform {
    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Uniform {
    type Context<'a> = UniformCreationContext<'a>;
    type Id = UniformId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        let mut content_changed = false;
        for field in &mut self.fields {
            content_changed |= field.refresh(&tracker, ctx)?;
        }

        if !self.dirty && !content_changed {
            return Ok(None);
        }

        let content = cast_fields(&self.fields);

        self.dirty = false;

        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;
        match self
            .buffer
            .write(ctx.device, ctx.queue, &self.label, &content, usage)?
        {
            ChangeResult::Uploaded => Ok(None),
            ChangeResult::Recreated => Ok(Some(ProjectEvent::UniformRecreated(id))),
        }
    }
}

/// Casts all field data to a packed byte buffer respecting std140 alignment rules.
fn cast_fields(fields: &[UniformField]) -> Vec<u8> {
    let mut buf = vec![];
    let mut struct_align = 1;

    for field in fields {
        let value = field.source().get_value();

        let (align, _) = value.kind().layout();

        struct_align = std::cmp::max(struct_align, align);

        let new_len = buf.len().next_multiple_of(align);
        buf.resize(new_len, 0);

        value.write_to(&mut buf);
    }

    let final_len = buf.len().next_multiple_of(struct_align);
    buf.resize(final_len, 0);

    buf
}

impl UniformField {
    pub fn new(label: impl Into<String>, source: UniformFieldSource) -> Self {
        Self {
            id: fastrand::usize(..),
            label: label.into(),
            source,
            dirty: false,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn source(&self) -> &UniformFieldSource {
        &self.source
    }

    fn refresh(
        &mut self,
        recreate_tracker: &RecreateTracker,
        context: &mut UniformCreationContext<'_>,
    ) -> AppResult<bool> {
        match &mut self.source {
            UniformFieldSource::UserDefined(_) => Ok(false), // Nothing to refresh
            UniformFieldSource::Camera {
                camera_id,
                field,
                current_value,
            } => {
                let Some(camera_id) = *camera_id else {
                    return Ok(false);
                };

                if !self.dirty && !recreate_tracker.happened(ProjectEvent::CameraUpdated(camera_id))
                {
                    return Ok(false);
                }

                let camera = context.cameras.get(camera_id)?;

                self.dirty = false;

                *current_value = field.compute(camera);
                Ok(true)
            }
        }
    }
}

impl UniformFieldSource {
    pub fn new_user_defined(data: UniformFieldData) -> Self {
        Self::UserDefined(data)
    }

    pub fn new_camera_sourced(camera_id: Option<CameraId>, field: CameraField) -> Self {
        Self::Camera {
            camera_id,
            field,
            current_value: field.default_data(),
        }
    }

    pub fn get_value(&self) -> &UniformFieldData {
        match self {
            UniformFieldSource::UserDefined(data) => data,
            UniformFieldSource::Camera { current_value, .. } => current_value,
        }
    }
}

impl UniformFieldData {
    pub fn from_kind(kind: UniformFieldDataKind) -> Self {
        match kind {
            UniformFieldDataKind::Vec2f => UniformFieldData::Vec2f([0.0; 2]),
            UniformFieldDataKind::Vec3f => UniformFieldData::Vec3f([0.0; 3]),
            UniformFieldDataKind::Vec4f => UniformFieldData::Vec4f([0.0; 4]),
            UniformFieldDataKind::Rgb => UniformFieldData::Rgb([1.0; 3]),
            UniformFieldDataKind::Rgba => UniformFieldData::Rgba([1.0; 4]),
            UniformFieldDataKind::Mat4x4f => UniformFieldData::Mat4x4f([[0.0; 4]; 4]),
        }
    }

    pub fn kind(&self) -> UniformFieldDataKind {
        match self {
            UniformFieldData::Vec2f(_) => UniformFieldDataKind::Vec2f,
            UniformFieldData::Vec3f(_) => UniformFieldDataKind::Vec3f,
            UniformFieldData::Vec4f(_) => UniformFieldDataKind::Vec4f,
            UniformFieldData::Rgb(_) => UniformFieldDataKind::Rgb,
            UniformFieldData::Rgba(_) => UniformFieldDataKind::Rgba,
            UniformFieldData::Mat4x4f(_) => UniformFieldDataKind::Mat4x4f,
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

impl UniformFieldDataKind {
    pub fn layout(&self) -> (usize, usize) {
        match self {
            UniformFieldDataKind::Vec2f => (8, 8),
            UniformFieldDataKind::Vec3f
            | UniformFieldDataKind::Rgb
            | UniformFieldDataKind::Vec4f
            | UniformFieldDataKind::Rgba => (16, 16),
            UniformFieldDataKind::Mat4x4f => (16, 64),
        }
    }

    pub fn wgsl_type_label(&self) -> &'static str {
        match self {
            UniformFieldDataKind::Vec2f => "vec2<f32>",
            UniformFieldDataKind::Vec3f => "vec3<f32>",
            UniformFieldDataKind::Vec4f => "vec4<f32>",
            UniformFieldDataKind::Rgb => "vec3<f32>",
            UniformFieldDataKind::Rgba => "vec4<f32>",
            UniformFieldDataKind::Mat4x4f => "mat4x4<f32>",
        }
    }
}

impl std::hash::Hash for UniformField {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
