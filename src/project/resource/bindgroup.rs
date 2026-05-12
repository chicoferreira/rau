use egui_dnd::utils::shift_vec;
use serde::{Deserialize, Serialize};
use std::task::Poll;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, Creatable, ProjectResource, SamplerId, TextureViewId, UniformId,
        resource::{sampler::Sampler, texture_view::TextureView, uniform::Uniform},
        storage::RuntimeStorage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

pub struct BindGroupCreationContext<'a> {
    pub runtime_uniforms: &'a RuntimeStorage<Uniform>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_samplers: &'a RuntimeStorage<Sampler>,
    pub device: &'a wgpu::Device,
    pub limits: &'a wgpu::Limits,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindGroup {
    label: String,
    entries: Vec<BindGroupEntry>,
    #[serde(skip)]
    revision: Revision,
}

pub struct BindGroupRuntime {
    layout: wgpu::BindGroupLayout,
    inner: wgpu::BindGroup,
}

#[derive(Default)]
pub enum BindGroupJob {
    #[default]
    Start,
    Validation(BindGroupRuntime, AsyncJob<AppResult<()>>),
}

pub type BindGroupEntryId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindGroupEntry {
    // Used for stability in bind group entry reordering
    pub id: BindGroupEntryId,
    pub visibility: wgpu::ShaderStages,
    pub resource: BindGroupResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum BindGroupResource {
    Texture {
        texture_view_id: Option<TextureViewId>,
        // These two fields are used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        view_dimension: wgpu::TextureViewDimension,
        sample_type: wgpu::TextureSampleType,
    },
    StorageTexture {
        texture_view_id: Option<TextureViewId>,
        access: wgpu::StorageTextureAccess,
        view_dimension: wgpu::TextureViewDimension,
    },
    Sampler {
        sampler_id: Option<SamplerId>,
        // This field is used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        sampler_binding_type: wgpu::SamplerBindingType,
    },
    Uniform(Option<UniformId>),
}

impl BindGroup {
    pub fn new(label: impl Into<String>, entries: Vec<BindGroupEntry>) -> BindGroup {
        BindGroup {
            label: label.into(),
            entries,
            revision: Revision::default(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn entries(&self) -> &[BindGroupEntry] {
        &self.entries
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.revision.increase();
    }

    pub fn add_entry(&mut self, entry: BindGroupEntry) {
        self.entries.push(entry);
        self.revision.increase();
    }

    pub fn remove_entry(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
            self.revision.increase();
        }
    }

    pub fn update_entry(&mut self, index: usize, entry: BindGroupEntry) {
        if index < self.entries.len() {
            self.entries[index] = entry;
            self.revision.increase();
        }
    }

    fn create_layout_and_bind_group(
        ctx: &BindGroupCreationContext,
        label: &str,
        entries: &[BindGroupEntry],
    ) -> AppResult<(wgpu::BindGroupLayout, wgpu::BindGroup)> {
        let mut layout_entries = Vec::new();
        let mut group_entries = Vec::new();

        for (index, entry) in entries.iter().copied().enumerate() {
            let group_entry = entry.into_bind_group_entry(index as u32, ctx)?;
            let layout_entry = entry.into_bind_group_layout_entry(index as u32, ctx)?;
            layout_entries.push(layout_entry);
            group_entries.push(group_entry);
        }

        let device = ctx.device;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &group_entries,
        });

        Ok((layout, bind_group))
    }

    pub fn reorder_entries(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.entries);
        self.revision.increase();
    }
}

impl BindGroupRuntime {
    pub fn inner_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn inner(&self) -> &wgpu::BindGroup {
        &self.inner
    }
}

impl Creatable for BindGroup {
    fn create(label: String) -> Self {
        Self::new(label, vec![])
    }
}

impl ProjectResource for BindGroup {
    type Id = BindGroupId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl BindGroupEntry {
    pub fn new(resource: BindGroupResource, visibility: wgpu::ShaderStages) -> Self {
        Self {
            id: fastrand::u64(..),
            visibility,
            resource,
        }
    }

    pub fn new_vertex_fragment(resource: BindGroupResource) -> Self {
        Self::new(resource, wgpu::ShaderStages::VERTEX_FRAGMENT)
    }

    pub fn new_compute(resource: BindGroupResource) -> Self {
        Self::new(resource, wgpu::ShaderStages::COMPUTE)
    }

    fn into_bind_group_entry<'a>(
        &self,
        binding: u32,
        ctx: &'a BindGroupCreationContext<'a>,
    ) -> AppResult<wgpu::BindGroupEntry<'a>> {
        let resource = match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            }
            | BindGroupResource::StorageTexture {
                texture_view_id, ..
            } => {
                let texture_view_id = texture_view_id.ok_or(AppError::UninitializedFields)?;
                let texture_view_runtime = ctx.runtime_texture_views.get_init(texture_view_id)?;
                let inner = texture_view_runtime.inner();

                wgpu::BindingResource::TextureView(inner)
            }
            BindGroupResource::Sampler { sampler_id, .. } => {
                let sampler_id = sampler_id.ok_or(AppError::UninitializedFields)?;
                let sampler = ctx.runtime_samplers.get_init(sampler_id)?;
                wgpu::BindingResource::Sampler(sampler.inner())
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform_id = uniform_id.ok_or(AppError::UninitializedFields)?;
                let uniform = ctx.runtime_uniforms.get_init(uniform_id)?;
                uniform.buffer().inner().as_entire_binding()
            }
        };

        Ok(wgpu::BindGroupEntry { binding, resource })
    }

    fn into_bind_group_layout_entry(
        &self,
        binding: u32,
        ctx: &BindGroupCreationContext,
    ) -> AppResult<wgpu::BindGroupLayoutEntry> {
        let ty = self.resource.to_wgpu_binding_type(ctx)?;

        Ok(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: self.visibility,
            ty,
            count: None,
        })
    }

    fn resource_recreated(&self, tracker: &SyncTracker) -> bool {
        match self.resource {
            BindGroupResource::Texture {
                texture_view_id: Some(texture_view_id),
                ..
            } => tracker.was_changed(texture_view_id),
            BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                ..
            } => tracker.was_changed(sampler_id),
            BindGroupResource::Uniform(Some(uniform_id)) => tracker.was_changed(uniform_id),
            BindGroupResource::StorageTexture {
                texture_view_id: Some(texture_view_id),
                ..
            } => tracker.was_changed(texture_view_id),
            _ => false,
        }
    }
}

impl BindGroupResource {
    fn to_wgpu_binding_type(self, ctx: &BindGroupCreationContext) -> AppResult<wgpu::BindingType> {
        Ok(match self {
            BindGroupResource::Texture {
                view_dimension,
                sample_type,
                ..
            } => wgpu::BindingType::Texture {
                sample_type,
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
            BindGroupResource::StorageTexture {
                texture_view_id,
                access,
                view_dimension,
            } => {
                if ctx.limits.max_storage_textures_per_shader_stage == 0 {
                    return Err(AppError::UnsupportedRendererFeature {
                        feature: "Storage Textures",
                    });
                }

                let texture_view_id = texture_view_id.ok_or(AppError::UninitializedFields)?;
                let texture_view_runtime = ctx.runtime_texture_views.get_init(texture_view_id)?;
                let format = texture_view_runtime.inner().texture().format();

                wgpu::BindingType::StorageTexture {
                    access,
                    view_dimension,
                    format,
                }
            }
        })
    }
}

impl SyncResource for BindGroup {
    type Context<'a> = BindGroupCreationContext<'a>;
    type Runtime = BindGroupRuntime;
    type Job = BindGroupJob;

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            BindGroupJob::Start => {
                let scope = WgpuErrorScope::push(ctx.device);
                let (layout, inner) =
                    Self::create_layout_and_bind_group(ctx, &self.label, &self.entries)?;

                let runtime = Self::Runtime { layout, inner };

                Ok(SyncOutcome::Pending(BindGroupJob::Validation(
                    runtime,
                    scope.pop(),
                )))
            }
            BindGroupJob::Validation(runtime, mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(BindGroupJob::Validation(
                    runtime, future,
                ))),
            },
        }
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.resource_recreated(tracker))
    }
}
