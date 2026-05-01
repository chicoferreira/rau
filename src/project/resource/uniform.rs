use egui_dnd::utils::shift_vec;

pub mod camera;
#[cfg(test)]
mod tests;

use crate::{
    error::{AppError, AppResult},
    project::{
        CameraId, Creatable, ProjectResource, UniformId,
        resource::{camera::Camera, uniform::camera::CameraField},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils::resizable_buffer::{ChangeResult, ResizableBuffer},
};

pub struct UniformCreationContext<'a> {
    pub cameras: &'a Storage<Camera>,
    pub cameras_runtime: &'a RuntimeStorage<Camera>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct Uniform {
    pub label: String,
    fields: Vec<UniformField>,
    revision: Revision,
}

pub struct UniformRuntime {
    fields: Vec<UniformRuntimeField>,
    buffer: ResizableBuffer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UniformRuntimeField {
    data: UniformFieldData,
}

type UniformFieldId = usize;

#[derive(Debug)]
pub struct UniformField {
    label: String,
    id: UniformFieldId, // Used for stability in reordering
    source: UniformFieldSource,
}

#[derive(Debug, Clone)]
pub enum UniformFieldSource {
    UserDefined(UniformFieldData),
    Camera {
        camera_id: Option<CameraId>,
        field: CameraField,
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
    pub fn new(label: impl Into<String>, fields: Vec<UniformField>) -> Uniform {
        let label = label.into();
        Uniform {
            label,
            fields,
            revision: Revision::default(),
        }
    }

    pub fn fields(&self) -> &[UniformField] {
        &self.fields
    }

    pub fn get_field(&self, index: usize) -> Option<&UniformField> {
        self.fields.get(index)
    }

    pub fn add_field(&mut self, field: UniformField) {
        self.fields.push(field);
        self.revision.increase();
    }

    pub fn remove_field(&mut self, index: usize) {
        if index < self.fields.len() {
            self.fields.remove(index);
            self.revision.increase();
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
            self.revision.increase();
        }
    }

    pub fn reorder_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.fields);
        self.revision.increase();
    }

    fn runtime_fields(
        &self,
        ctx: &UniformCreationContext<'_>,
    ) -> AppResult<Vec<UniformRuntimeField>> {
        self.fields
            .iter()
            .map(|field| {
                Ok(UniformRuntimeField {
                    data: field.runtime_data(ctx)?,
                })
            })
            .collect()
    }
}

impl UniformRuntime {
    pub fn buffer(&self) -> &ResizableBuffer {
        &self.buffer
    }

    pub fn layout(&self) -> (usize, usize) {
        let mut size = 0usize;
        let mut struct_align = 1usize;

        for field in &self.fields {
            let (align, field_size) = field.data.kind().layout();
            struct_align = struct_align.max(align);
            size = size.next_multiple_of(align);
            size += field_size;
        }

        (size.next_multiple_of(struct_align), struct_align)
    }

    pub fn fields(&self) -> &[UniformRuntimeField] {
        &self.fields
    }
}

impl UniformRuntimeField {
    pub fn data(&self) -> &UniformFieldData {
        &self.data
    }
}

impl Creatable for Uniform {
    const DEFAULT_LABEL: &'static str = "Uniform";

    fn create(label: String) -> Self {
        Uniform::new(label, vec![])
    }
}

impl ProjectResource for Uniform {
    type Id = UniformId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for Uniform {
    type Context<'a> = UniformCreationContext<'a>;
    type Runtime = UniformRuntime;

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let fields = self.runtime_fields(ctx)?;
        let content = cast_fields(&fields);
        let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;

        match previous {
            Some(mut runtime) => {
                let content_changed = runtime.fields != fields;
                runtime.fields = fields;

                if !content_changed {
                    return Ok(SyncOutcome::Unchanged(runtime));
                }

                match runtime
                    .buffer
                    .write(ctx.device, ctx.queue, &self.label, &content, usage)
                {
                    ChangeResult::Uploaded => Ok(SyncOutcome::Unchanged(runtime)),
                    ChangeResult::Recreated => Ok(SyncOutcome::Changed(runtime)),
                }
            }
            None => {
                let buffer = ResizableBuffer::new(ctx.device, &self.label, usage, &content);
                Ok(SyncOutcome::Changed(UniformRuntime { fields, buffer }))
            }
        }
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.fields
            .iter()
            .any(|field| field.needs_rebuild_from_others(tracker))
    }
}

/// Casts all field data to a packed byte buffer respecting std140 alignment rules.
fn cast_fields(fields: &[UniformRuntimeField]) -> Vec<u8> {
    let mut buf = vec![];
    let mut struct_align = 1;

    for field in fields {
        let value = &field.data;

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
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn source(&self) -> &UniformFieldSource {
        &self.source
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        match &self.source {
            UniformFieldSource::UserDefined(_) => false,
            UniformFieldSource::Camera { camera_id, .. } => {
                let Some(camera_id) = *camera_id else {
                    return false;
                };

                tracker.was_changed(camera_id)
            }
        }
    }

    fn runtime_data(&self, context: &UniformCreationContext<'_>) -> AppResult<UniformFieldData> {
        match &self.source {
            UniformFieldSource::UserDefined(data) => Ok(data.clone()),
            UniformFieldSource::Camera { camera_id, field } => {
                let camera_id = (*camera_id).ok_or(AppError::UninitializedFields)?;
                let camera = context.cameras.get(camera_id)?;
                let camera_runtime = context.cameras_runtime.get_init(camera_id)?;
                Ok(field.compute(camera, camera_runtime))
            }
        }
    }
}

impl UniformFieldSource {
    pub fn new_user_defined(data: UniformFieldData) -> Self {
        Self::UserDefined(data)
    }

    pub fn new_camera_sourced(camera_id: Option<CameraId>, field: CameraField) -> Self {
        Self::Camera { camera_id, field }
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
