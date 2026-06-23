use instant::Duration;
use serde::{Deserialize, Serialize};
use std::task::Poll;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, ComputePassId, Creatable, ProjectResource, ShaderId,
        resource::{bindgroup::BindGroup, shader::Shader},
        storage::RuntimeStorage,
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
    work_groups: WorkGroups,
    #[serde(default)]
    dispatch: DispatchPolicy,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorkGroups(u32, u32, u32);

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
            work_groups: WorkGroups::new(1, 1, 1),
            dispatch: DispatchPolicy::default(),
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
        work_groups: WorkGroups,
        dispatch: DispatchPolicy,
    ) -> Self {
        Self {
            label: label.into(),
            bind_groups,
            shader,
            work_groups,
            dispatch,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn bind_groups() -> &[BindGroupId];
        pub fn shader() -> Option<ShaderId>;
        pub fn work_groups() -> WorkGroups;
        pub fn dispatch() -> DispatchPolicy;
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_shader(shader: Option<ShaderId>);
        pub fn set_bind_groups(bind_groups: Vec<BindGroupId>);
        pub fn set_work_groups(work_groups: WorkGroups);
        pub fn set_dispatch(dispatch: DispatchPolicy);
    }

    /// Whether any of this pass's inputs changed their data this frame. Used by
    /// [`DispatchPolicy::OnChange`] to decide whether to re-dispatch.
    pub fn inputs_changed(&self, tracker: &SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_data_changed(id))
            || self
                .bind_groups
                .iter()
                .any(|id| tracker.was_data_changed(*id))
    }

    /// Encodes one dispatch of this pass into `encoder`.
    ///
    /// Returns `Ok(true)` once fully encoded, or `Ok(false)` if a bind group is
    /// still rebuilding (the caller should try again next frame). Mirrors
    /// [`crate::project::resource::render_pass::RenderPass::submit`].
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        runtime: &ComputePassRuntime,
        runtime_bind_groups: &RuntimeStorage<BindGroup>,
    ) -> AppResult<bool> {
        let mut bind_groups = Vec::with_capacity(self.bind_groups.len());
        for id in self.bind_groups.iter().copied() {
            let Some(bind_group) = runtime_bind_groups.get_init(id)? else {
                return Ok(false); // pending: a bind group is still rebuilding
            };
            bind_groups.push(bind_group);
        }

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&format!("{} (Compute Pass)", self.label)),
            timestamp_writes: None,
        });

        pass.set_pipeline(runtime.pipeline());
        for (index, bind_group) in bind_groups.into_iter().enumerate() {
            pass.set_bind_group(index as u32, bind_group.inner(), &[]);
        }

        let (x, y, z) = self.work_groups().into();
        pass.dispatch_workgroups(x, y, z);

        Ok(true)
    }
}

impl WorkGroups {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self(x.max(1), y.max(1), z.max(1))
    }

    pub fn into(self) -> (u32, u32, u32) {
        (self.0, self.1, self.2)
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
