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
    resource_getters, resource_setters,
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
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct BindGroupRuntime {
    layout: wgpu::BindGroupLayout,
    layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
    inner: wgpu::BindGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BindGroupEntry {
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
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn entries() -> &[BindGroupEntry];
    }

    resource_setters! {
        increases: [project_revision, runtime_revision];
        pub fn set_label(label: String);
        pub fn set_entries(entries: Vec<BindGroupEntry>);
    }

    fn resolve_entries<'a>(
        ctx: &'a BindGroupCreationContext<'a>,
        entries: &[BindGroupEntry],
    ) -> AppResult<
        Option<(
            Vec<wgpu::BindGroupLayoutEntry>,
            Vec<wgpu::BindGroupEntry<'a>>,
        )>,
    > {
        let mut layout_entries = Vec::new();
        let mut group_entries = Vec::new();

        for (index, entry) in entries.iter().copied().enumerate() {
            let Some(group_entry) = entry.into_bind_group_entry(index as u32, ctx)? else {
                return Ok(None);
            };
            let Some(layout_entry) = entry.into_bind_group_layout_entry(index as u32, ctx)? else {
                return Ok(None);
            };
            layout_entries.push(layout_entry);
            group_entries.push(group_entry);
        }

        Ok(Some((layout_entries, group_entries)))
    }

    fn create_layout_and_bind_group(
        ctx: &BindGroupCreationContext,
        label: &str,
        layout_entries: &[wgpu::BindGroupLayoutEntry],
        group_entries: &[wgpu::BindGroupEntry<'_>],
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let device = ctx.device;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: layout_entries,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: group_entries,
        });

        (layout, bind_group)
    }
}

impl BindGroupRuntime {
    pub fn inner_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn layout_entries(&self) -> &[wgpu::BindGroupLayoutEntry] {
        &self.layout_entries
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

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl BindGroupEntry {
    pub fn new(resource: BindGroupResource, visibility: wgpu::ShaderStages) -> Self {
        Self {
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
    ) -> AppResult<Option<wgpu::BindGroupEntry<'a>>> {
        let resource = match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            }
            | BindGroupResource::StorageTexture {
                texture_view_id, ..
            } => {
                let texture_view_id = texture_view_id.ok_or(AppError::uninit_field(format!(
                    "Binding {binding} Texture View Id"
                )))?;
                let Some(texture_view_runtime) =
                    ctx.runtime_texture_views.get_init(texture_view_id)?
                else {
                    return Ok(None);
                };
                let inner = texture_view_runtime.inner();

                wgpu::BindingResource::TextureView(inner)
            }
            BindGroupResource::Sampler { sampler_id, .. } => {
                let sampler_id = sampler_id.ok_or(AppError::uninit_field(format!(
                    "Binding {binding} Sampler Id"
                )))?;
                let Some(sampler) = ctx.runtime_samplers.get_init(sampler_id)? else {
                    return Ok(None);
                };
                wgpu::BindingResource::Sampler(sampler.inner())
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform_id = uniform_id.ok_or(AppError::uninit_field(format!(
                    "Binding {binding} Uniform Id"
                )))?;
                let Some(uniform) = ctx.runtime_uniforms.get_init(uniform_id)? else {
                    return Ok(None);
                };
                uniform.buffer().inner().as_entire_binding()
            }
        };

        Ok(Some(wgpu::BindGroupEntry { binding, resource }))
    }

    fn into_bind_group_layout_entry(
        &self,
        binding: u32,
        ctx: &BindGroupCreationContext,
    ) -> AppResult<Option<wgpu::BindGroupLayoutEntry>> {
        let Some(ty) = self.resource.to_wgpu_binding_type(binding, ctx)? else {
            return Ok(None);
        };

        Ok(Some(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: self.visibility,
            ty,
            count: None,
        }))
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
    fn to_wgpu_binding_type(
        self,
        binding: u32,
        ctx: &BindGroupCreationContext,
    ) -> AppResult<Option<wgpu::BindingType>> {
        Ok(match self {
            BindGroupResource::Texture {
                view_dimension,
                sample_type,
                ..
            } => Some(wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled: false,
            }),
            BindGroupResource::Sampler {
                sampler_binding_type,
                ..
            } => Some(wgpu::BindingType::Sampler(sampler_binding_type)),
            BindGroupResource::Uniform(_) => Some(wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            }),
            BindGroupResource::StorageTexture {
                texture_view_id,
                access,
                view_dimension,
            } => {
                if ctx.limits.max_storage_textures_per_shader_stage == 0 {
                    return Err(AppError::UnsupportedRendererFeature("Storage Textures"));
                }

                let texture_view_id = texture_view_id.ok_or(AppError::uninit_field(format!(
                    "Binding {binding} Texture View Id"
                )))?;
                let Some(texture_view_runtime) =
                    ctx.runtime_texture_views.get_init(texture_view_id)?
                else {
                    return Ok(None);
                };
                let format = texture_view_runtime.inner().texture().format();

                Some(wgpu::BindingType::StorageTexture {
                    access,
                    view_dimension,
                    format,
                })
            }
        })
    }
}

#[derive(Default)]
pub enum BindGroupJob {
    #[default]
    Start,
    Validation(BindGroupRuntime, AsyncJob<AppResult<()>>),
}

impl SyncResource for BindGroup {
    type Context<'a> = BindGroupCreationContext<'a>;
    type Runtime = BindGroupRuntime;
    type Job = BindGroupJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            BindGroupJob::Start => {
                let Some((layout_entries, group_entries)) =
                    Self::resolve_entries(ctx, &self.entries)?
                else {
                    return Ok(SyncOutcome::Pending(BindGroupJob::Start));
                };

                let scope = WgpuErrorScope::push(ctx.device);
                let (layout, inner) = Self::create_layout_and_bind_group(
                    ctx,
                    &self.label,
                    &layout_entries,
                    &group_entries,
                );

                let runtime = Self::Runtime {
                    layout,
                    layout_entries,
                    inner,
                };

                self.sync(ctx, None, BindGroupJob::Validation(runtime, scope.pop()))
            }
            BindGroupJob::Validation(runtime, mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(BindGroupJob::Validation(
                    runtime, future,
                ))),
            },
        }
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.resource_recreated(tracker))
    }
}
