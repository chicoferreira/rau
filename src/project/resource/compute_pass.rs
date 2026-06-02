use itertools::Itertools;
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
    bind_groups: Vec<Option<BindGroupId>>,
    shader: Option<ShaderId>,
    work_groups: WorkGroups,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorkGroups(u32, u32, u32);

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

#[derive(Default)]
pub enum ComputePassJob {
    #[default]
    Start,
    Validation(AsyncJob<AppResult<()>>),
}

impl Creatable for ComputePass {
    fn create(label: String) -> Self {
        Self {
            label,
            bind_groups: Default::default(),
            shader: Default::default(),
            work_groups: WorkGroups::new(1, 1, 1),
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }
}

impl ComputePass {
    pub fn new(
        label: impl Into<String>,
        bind_groups: Vec<Option<BindGroupId>>,
        shader: Option<ShaderId>,
        work_groups: WorkGroups,
    ) -> Self {
        Self {
            label: label.into(),
            bind_groups,
            shader,
            work_groups,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn bind_groups() -> &[Option<BindGroupId>];
        pub fn shader() -> Option<ShaderId>;
        pub fn work_groups() -> WorkGroups;
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_shader(shader: Option<ShaderId>);
        pub fn set_bind_groups(bind_groups: Vec<Option<BindGroupId>>);
        pub fn set_work_groups(work_groups: WorkGroups);
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
    type Runtime = ();
    type Job = ComputePassJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn needs_rebuild(&self, _: Self::Id, _: &Self::Context<'_>, tracker: &SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_changed(id))
            || self
                .bind_groups
                .iter()
                .filter_map(|bind_group_id| *bind_group_id)
                .any(|id| tracker.was_changed(id))
    }

    fn sync<'a>(
        &self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            ComputePassJob::Start => {
                let limits = ctx.device.limits();

                if limits.max_compute_workgroups_per_dimension == 0 {
                    return Err(AppError::UnsupportedRendererFeature("Compute Passes"));
                }

                let mut bind_groups = vec![];
                for (i, bind_group_id) in self.bind_groups.iter().copied().enumerate() {
                    let id = bind_group_id
                        .ok_or(AppError::uninit_field(format!("Bind Group {i} Id")))?;

                    let Some(bind_group_runtime) = ctx.runtime_bind_groups.get_init(id)? else {
                        return Ok(SyncOutcome::Pending(ComputePassJob::Start));
                    };

                    bind_groups.push(bind_group_runtime);
                }

                let bind_group_layouts = bind_groups
                    .iter()
                    .map(|bg| Some(bg.inner_layout()))
                    .collect_vec();

                validate_bind_group_layouts(&bind_group_layouts, &limits)?;

                let shader_id = self.shader.ok_or(AppError::uninit_field("Shader"))?;

                let Some(shader_runtime) = ctx.runtime_shaders.get_init(shader_id)? else {
                    return Ok(SyncOutcome::Pending(ComputePassJob::Start));
                };

                let scope = WgpuErrorScope::push(ctx.device);

                let pipeline_layout =
                    ctx.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some(&format!("{} (Pipeline Layout)", self.label)),
                            bind_group_layouts: &bind_group_layouts,
                            immediate_size: 0,
                        });

                let pipeline =
                    ctx.device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some(&format!("{} (Compute Pipeline)", self.label)),
                            layout: Some(&pipeline_layout),
                            module: shader_runtime.inner(),
                            entry_point: None,
                            compilation_options: Default::default(),
                            cache: None,
                        });

                let mut pass = ctx
                    .encoder
                    .begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(&format!("{} (Compute Pass)", self.label)),
                        timestamp_writes: None,
                    });

                pass.set_pipeline(&pipeline);
                for (index, bind_group_runtime) in bind_groups.into_iter().enumerate() {
                    pass.set_bind_group(index as u32, bind_group_runtime.inner(), &[]);
                }

                let (x, y, z) = self.work_groups().into();
                pass.dispatch_workgroups(x, y, z);

                drop(pass);

                self.sync(id, ctx, None, ComputePassJob::Validation(scope.pop()))
            }
            ComputePassJob::Validation(mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(())),
                Poll::Pending => Ok(SyncOutcome::Pending(ComputePassJob::Validation(future))),
            },
        }
    }
}
