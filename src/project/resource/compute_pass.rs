use instant::Duration;
use serde::{Deserialize, Serialize};
use std::task::Poll;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, ComputePassId, Creatable, DimensionId, ProjectResource, ShaderId,
        resource::{
            bindgroup::BindGroup,
            dimension::{Axis, Dimension, DimensionRef},
            shader::Shader,
        },
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::{
        async_job::AsyncJob, validate_bind_group_layouts::validate_bind_group_layouts,
        wgpu_error_scope::WgpuErrorScope,
    },
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputePass {
    label: String,
    bind_groups: Vec<BindGroupId>,
    shader: Option<ShaderId>,
    #[serde(alias = "workGroups")]
    dispatch_size: DispatchSize,
    #[serde(default)]
    dispatch_policy: DispatchPolicy,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

/// The size of a dispatch, counted in whichever unit [`DispatchUnit`] selects.
///
/// Resolves to the three workgroup counts handed to
/// [`wgpu::ComputePass::dispatch_workgroups`].
///
/// Each axis is either a constant or read from a [`Dimension`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchSize {
    pub x: WorkSize,
    pub y: WorkSize,
    pub z: WorkSize,
    pub unit: DispatchUnit,
}

/// What the values in a [`DispatchSize`] count.
///
/// Every workgroup runs `@workgroup_size` invocations, so the two units are one
/// multiplication apart. The unit decides which of them the work sizes are.
///
/// Against a shader declaring `@workgroup_size(16, 16, 1)`, these dispatch the
/// same 128x128 invocations:
///
/// - `new_fixed(8, 8, 1, Workgroup)`
/// - `new_fixed(128, 128, 1, Invocation { workgroup_size: [16, 16, 1] })`
///
/// [`Invocation`](Self::Invocation) is the practical choice when an axis reads
/// from a [`Dimension`] and the compute pass is expected to run for every pixel
/// of that dimension.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DispatchUnit {
    /// Workgroups, passed to `dispatch_workgroups` unchanged.
    #[default]
    Workgroup,
    /// Invocations, divided by `workgroup_size` to get the workgroup counts.
    ///
    /// The division rounds up, so the last workgroup along each axis runs past
    /// the requested range; the shader needs its own bounds check on
    /// `@builtin(global_invocation_id)`.
    ///
    /// `workgroup_size` has to be kept in step with the `@workgroup_size`
    /// for this to make sense.
    Invocation {
        // TODO: grab from shader via reflection instead of having the user specify it
        #[serde(alias = "workgroup_size")]
        workgroup_size: [u32; 3],
    },
}

/// One axis of a [`DispatchSize`]: a constant, or the width or height of a
/// [`Dimension`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkSize {
    Fixed(u32),
    Dimension(DimensionRef),
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DispatchPolicy {
    /// Dispatch only on a frame where one of the pass's inputs changed, or the
    /// pipeline was just (re)built.
    #[default]
    OnChange,
    /// Dispatch on every rendered frame.
    EveryFrame,
    /// Dispatch at a fixed cadence, independent of the framerate.
    Periodic {
        #[serde(with = "duration_secs")]
        interval: Duration,
    },
}

/// Serializes a [`Duration`] as plain seconds (e.g. `0.05`) instead of the
/// verbose `{ secs, nanos }` form, keeping `project.json` readable.
mod duration_secs {
    use instant::Duration;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(duration.as_secs_f32())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
        Ok(Duration::from_secs_f32(f32::deserialize(deserializer)?))
    }
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

pub struct ComputePassRuntime {
    pipeline: wgpu::ComputePipeline,
}

impl ComputePassRuntime {
    pub fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }
}

#[derive(Default)]
pub enum ComputePassJob {
    #[default]
    Start,
    Validation(AsyncJob<AppResult<()>>, ComputePassRuntime),
}

impl Creatable for ComputePass {
    fn create(label: String) -> Self {
        Self {
            label,
            bind_groups: Default::default(),
            shader: Default::default(),
            dispatch_size: DispatchSize::new_fixed(1, 1, 1, DispatchUnit::Workgroup),
            dispatch_policy: DispatchPolicy::default(),
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }
}

impl ComputePass {
    pub fn new(
        label: impl Into<String>,
        bind_groups: Vec<BindGroupId>,
        shader: Option<ShaderId>,
        dispatch_size: DispatchSize,
        dispatch_policy: DispatchPolicy,
    ) -> Self {
        Self {
            label: label.into(),
            bind_groups,
            shader,
            dispatch_size,
            dispatch_policy,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn bind_groups() -> &[BindGroupId];
        pub fn shader() -> Option<ShaderId>;
        pub fn dispatch_size() -> DispatchSize;
        pub fn dispatch_policy() -> DispatchPolicy;
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_shader(shader: Option<ShaderId>);
        pub fn set_bind_groups(bind_groups: Vec<BindGroupId>);
        pub fn set_dispatch_size(dispatch_size: DispatchSize);
        pub fn set_dispatch(dispatch_policy: DispatchPolicy);
    }

    /// Whether any of this pass's inputs changed their data this frame. Used by
    /// [`DispatchPolicy::OnChange`] to decide whether to re-dispatch.
    pub fn inputs_changed(&self, tracker: &SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_data_changed(id))
            || self
                .bind_groups
                .iter()
                .any(|id| tracker.was_data_changed(*id))
            || self
                .dispatch_size
                .dimension_ids()
                .any(|id| tracker.was_data_changed(id))
    }

    /// Encodes one dispatch of this pass into `encoder`.
    /// The pass is measured with a GPU timer query, so it appears on the frame profiler.
    ///
    /// Returns `Ok(true)` once fully encoded, or `Ok(false)` if a bind group is
    /// still rebuilding (the caller should try again next frame). Mirrors
    /// [`crate::project::resource::render_pass::RenderPass::submit`].
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        gpu_profiler: &wgpu_profiler::GpuProfiler,
        runtime: &ComputePassRuntime,
        runtime_bind_groups: &RuntimeStorage<BindGroup>,
        dimensions: &Storage<Dimension>,
    ) -> AppResult<bool> {
        let mut bind_groups = Vec::with_capacity(self.bind_groups.len());
        for id in self.bind_groups.iter().copied() {
            let Some(bind_group) = runtime_bind_groups.get_init(id)? else {
                return Ok(false); // pending: a bind group is still rebuilding
            };
            bind_groups.push(bind_group);
        }

        let label = format!("{} (Compute Pass)", self.label);
        let query = gpu_profiler.begin_pass_query(label.clone(), encoder);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&label),
            timestamp_writes: query.compute_pass_timestamp_writes(),
        });

        pass.set_pipeline(runtime.pipeline());
        for (index, bind_group) in bind_groups.into_iter().enumerate() {
            pass.set_bind_group(index as u32, bind_group.inner(), &[]);
        }

        let (x, y, z) = self.dispatch_size().into_work_groups(dimensions)?;
        pass.dispatch_workgroups(x, y, z);
        drop(pass);

        gpu_profiler.end_query(encoder, query);

        Ok(true)
    }
}

impl DispatchSize {
    pub fn new_fixed(x: u32, y: u32, z: u32, unit: DispatchUnit) -> Self {
        Self {
            x: WorkSize::Fixed(x),
            y: WorkSize::Fixed(y),
            z: WorkSize::Fixed(z),
            unit,
        }
    }

    pub fn new_dimension(dimension: DimensionId, z: u32, unit: DispatchUnit) -> Self {
        let axis = |axis| {
            WorkSize::Dimension(DimensionRef {
                id: Some(dimension),
                axis,
            })
        };

        Self {
            x: axis(Axis::Width),
            y: axis(Axis::Height),
            z: WorkSize::Fixed(z),
            unit,
        }
    }

    /// The dimensions the axes read, skipping the fixed and the unset ones.
    pub fn dimension_ids(&self) -> impl Iterator<Item = DimensionId> {
        [self.x, self.y, self.z]
            .into_iter()
            .filter_map(|work_size| match work_size {
                WorkSize::Fixed(_) => None,
                WorkSize::Dimension(dimension_ref) => dimension_ref.id,
            })
    }

    pub fn into_work_groups(self, dimensions: &Storage<Dimension>) -> AppResult<(u32, u32, u32)> {
        let x = self.x.resolve(dimensions)?;
        let y = self.y.resolve(dimensions)?;
        let z = self.z.resolve(dimensions)?;

        Ok(match self.unit {
            DispatchUnit::Workgroup => (x, y, z),
            DispatchUnit::Invocation {
                workgroup_size: [wx, wy, wz],
            } => (
                x.div_ceil(wx.max(1)), // `max(1)` avoids dividing by zero
                y.div_ceil(wy.max(1)),
                z.div_ceil(wz.max(1)),
            ),
        })
    }
}

impl WorkSize {
    pub fn resolve(&self, dimensions: &Storage<Dimension>) -> AppResult<u32> {
        match self {
            WorkSize::Fixed(value) => Ok(*value),
            WorkSize::Dimension(dimension_ref) => dimension_ref.resolve(dimensions),
        }
    }
}

impl ProjectResource for ComputePass {
    type Id = ComputePassId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for ComputePass {
    type Context<'a> = Context<'a>;
    type Runtime = ComputePassRuntime;
    type Job = ComputePassJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn needs_rebuild(&self, _: Self::Id, _: &Self::Context<'_>, tracker: &SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_recreated(id))
            || self.bind_groups.iter().any(|id| tracker.was_recreated(*id))
    }

    fn sync<'a>(
        &self,
        _id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        if let ComputePassJob::Validation(mut future, runtime) = job {
            return match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Recreated(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(ComputePassJob::Validation(
                    future, runtime,
                ))),
            };
        }

        let limits = ctx.device.limits();
        if limits.max_compute_workgroups_per_dimension == 0 {
            return Err(AppError::UnsupportedRendererFeature("Compute Passes"));
        }

        let mut bind_group_layouts = vec![];
        for id in self.bind_groups.iter().copied() {
            let Some(bind_group_runtime) = ctx.runtime_bind_groups.get_init(id)? else {
                return Ok(SyncOutcome::Pending(ComputePassJob::Start));
            };
            bind_group_layouts.push(Some(bind_group_runtime.inner_layout()));
        }

        validate_bind_group_layouts(&bind_group_layouts, &limits)?;

        let shader_id = self.shader.ok_or(AppError::uninit_field("Shader"))?;
        let Some(shader_runtime) = ctx.runtime_shaders.get_init(shader_id)? else {
            return Ok(SyncOutcome::Pending(ComputePassJob::Start));
        };

        let scope = WgpuErrorScope::push(ctx.device);

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{} (Pipeline Layout)", self.label)),
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            });

        let pipeline = ctx
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(&format!("{} (Compute Pipeline)", self.label)),
                layout: Some(&pipeline_layout),
                module: shader_runtime.inner(),
                entry_point: None,
                compilation_options: Default::default(),
                cache: None,
            });

        let runtime = ComputePassRuntime { pipeline };
        self.sync(
            _id,
            ctx,
            None,
            ComputePassJob::Validation(scope.pop(), runtime),
        )
    }
}
