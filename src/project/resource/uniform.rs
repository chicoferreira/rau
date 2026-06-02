use serde::{Deserialize, Serialize};
use std::task::Poll;

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
    resource_getters, resource_setters,
    utils::{
        async_job::AsyncJob,
        resizable_buffer::{ChangeResult, ResizableBuffer},
        wgpu_error_scope::WgpuErrorScope,
    },
};

pub struct UniformCreationContext<'a> {
    pub cameras: &'a Storage<Camera>,
    pub cameras_runtime: &'a RuntimeStorage<Camera>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub time: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Uniform {
    label: String,
    fields: Vec<UniformField>,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct UniformRuntime {
    fields: Vec<UniformRuntimeField>,
    buffer: ResizableBuffer,
}

#[derive(Default)]
pub enum UniformJob {
    #[default]
    Start,
    WaitingForResources {
        previous: Option<UniformRuntime>,
    },
    Validation(UniformRuntime, AsyncJob<AppResult<()>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct UniformRuntimeField {
    data: UniformFieldData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UniformField {
    label: String,
    source: UniformFieldSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum UniformFieldSource {
    UserDefined(UniformFieldData),
    Camera {
        camera_id: Option<CameraId>,
        field: CameraField,
    },
    Time,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "data_type", content = "data")]
pub enum UniformFieldData {
    Float(f32),
    Vec2f([f32; 2]),
    Vec3f([f32; 3]),
    Vec4f([f32; 4]),
    Rgb([f32; 3]),
    Rgba([f32; 4]),
    Mat4x4f([[f32; 4]; 4]),
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum UniformFieldDataKind {
    Float,
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
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn fields() -> &[UniformField];
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_fields(fields: Vec<UniformField>);
    }

    pub fn get_field(&self, index: usize) -> Option<&UniformField> {
        self.fields.get(index)
    }

    pub fn set_field_label(&mut self, index: usize, new_name: String) {
        if let Some(field) = self.fields.get_mut(index) {
            field.label = new_name;
        }
    }

    fn runtime_fields(
        &self,
        ctx: &UniformCreationContext<'_>,
    ) -> AppResult<Option<Vec<UniformRuntimeField>>> {
        let mut runtime_fields = Vec::with_capacity(self.fields.len());
        for (index, field) in self.fields.iter().enumerate() {
            let Some(data) = field.runtime_data(index, ctx)? else {
                return Ok(None);
            };
            runtime_fields.push(UniformRuntimeField { data });
        }
        Ok(Some(runtime_fields))
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
    fn create(label: String) -> Self {
        Uniform::new(label, vec![])
    }
}

impl ProjectResource for Uniform {
    type Id = UniformId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for Uniform {
    type Context<'a> = UniformCreationContext<'a>;
    type Runtime = UniformRuntime;
    type Job = UniformJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            UniformJob::Start => self.sync(ctx, None, UniformJob::WaitingForResources { previous }),
            UniformJob::WaitingForResources { previous } => {
                let Some(fields) = self.runtime_fields(ctx)? else {
                    return Ok(SyncOutcome::Pending(UniformJob::WaitingForResources {
                        previous,
                    }));
                };
                let content = cast_fields(&fields);
                let usage = wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST;

                match previous {
                    Some(mut runtime) => {
                        let content_changed = runtime.fields != fields;
                        runtime.fields = fields;

                        if !content_changed {
                            return Ok(SyncOutcome::Unchanged(runtime));
                        }

                        let scope = WgpuErrorScope::push(ctx.device);
                        match runtime.buffer.write(
                            ctx.device,
                            ctx.queue,
                            &self.label,
                            &content,
                            usage,
                        ) {
                            ChangeResult::Uploaded => Ok(SyncOutcome::Unchanged(runtime)),
                            ChangeResult::Recreated => {
                                self.sync(ctx, None, UniformJob::Validation(runtime, scope.pop()))
                            }
                        }
                    }
                    None => {
                        let scope = WgpuErrorScope::push(ctx.device);
                        let buffer = ResizableBuffer::new(ctx.device, &self.label, usage, &content);
                        let runtime = UniformRuntime { fields, buffer };
                        self.sync(ctx, None, UniformJob::Validation(runtime, scope.pop()))
                    }
                }
            }
            UniformJob::Validation(runtime, mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(UniformJob::Validation(
                    runtime, future,
                ))),
            },
        }
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
            // Time advances every frame, so the uniform must always be re-evaluated.
            UniformFieldSource::Time => true,
        }
    }

    fn runtime_data(
        &self,
        index: usize,
        context: &UniformCreationContext<'_>,
    ) -> AppResult<Option<UniformFieldData>> {
        match &self.source {
            UniformFieldSource::UserDefined(data) => Ok(Some(data.clone())),
            UniformFieldSource::Camera { camera_id, field } => {
                let camera_id = (*camera_id).ok_or(AppError::uninit_field(format!(
                    "Uniform Field {index} Camera Id",
                )))?;
                let camera = context.cameras.get(camera_id)?;
                let Some(camera_runtime) = context.cameras_runtime.get_init(camera_id)? else {
                    return Ok(None);
                };
                Ok(Some(field.compute(camera, camera_runtime)))
            }
            UniformFieldSource::Time => Ok(Some(UniformFieldData::Float(context.time))),
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

    pub fn new_time() -> Self {
        Self::Time
    }
}

impl UniformFieldData {
    pub fn from_kind(kind: UniformFieldDataKind) -> Self {
        match kind {
            UniformFieldDataKind::Float => UniformFieldData::Float(0.0),
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
            UniformFieldData::Float(_) => UniformFieldDataKind::Float,
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
            UniformFieldData::Float(v) => {
                buf.extend_from_slice(bytemuck::bytes_of(v));
            }
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
            UniformFieldDataKind::Float => (4, 4),
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
            UniformFieldDataKind::Float => "f32",
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
        self.label.hash(state);
    }
}
