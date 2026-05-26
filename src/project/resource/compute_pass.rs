use egui_dnd::utils::shift_vec;
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
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputePass {
    label: String,
    bind_groups: Vec<ComputePassBindGroupEntry>,
    shader: Option<ShaderId>,
    work_groups_x: u32,
    work_groups_y: u32,
    work_groups_z: u32,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub type ComputePassBindGroupEntryId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputePassBindGroupEntry {
    id: ComputePassBindGroupEntryId,
    bind_group_id: Option<BindGroupId>,
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub limits: &'a wgpu::Limits,
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
            work_groups_x: Default::default(),
            work_groups_y: Default::default(),
            work_groups_z: Default::default(),
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }
}

impl ComputePass {
    pub fn new(
        label: impl Into<String>,
        bind_groups: Vec<ComputePassBindGroupEntry>,
        shader: Option<ShaderId>,
        work_groups_x: u32,
        work_groups_y: u32,
        work_groups_z: u32,
    ) -> Self {
        Self {
            label: label.into(),
            bind_groups,
            shader,
            work_groups_x: work_groups_x.max(1),
            work_groups_y: work_groups_y.max(1),
            work_groups_z: work_groups_z.max(1),
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn bind_groups(&self) -> &[ComputePassBindGroupEntry] {
        &self.bind_groups
    }

    pub fn shader(&self) -> Option<ShaderId> {
        self.shader
    }

    pub fn work_groups(&self) -> (u32, u32, u32) {
        (self.work_groups_x, self.work_groups_y, self.work_groups_z)
    }

    pub fn set_label(&mut self, label: String) {
        if self.label != label {
            self.label = label;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn set_shader(&mut self, shader: Option<ShaderId>) {
        if self.shader != shader {
            self.shader = shader;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn set_work_groups(&mut self, x: u32, y: u32, z: u32) {
        let next = (x.max(1), y.max(1), z.max(1));
        let current = (self.work_groups_x, self.work_groups_y, self.work_groups_z);
        if current != next {
            self.work_groups_x = next.0;
            self.work_groups_y = next.1;
            self.work_groups_z = next.2;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn set_bind_group(&mut self, index: usize, bind_group: Option<BindGroupId>) {
        if let Some(current) = self.bind_groups.get_mut(index)
            && current.bind_group_id() != bind_group
        {
            current.set_bind_group_id(bind_group);
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn add_bind_group(&mut self, bind_group: Option<BindGroupId>) {
        self.bind_groups
            .push(ComputePassBindGroupEntry::new(bind_group));
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn remove_bind_group(&mut self, index: usize) {
        if index < self.bind_groups.len() {
            self.bind_groups.remove(index);
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn reorder_bind_groups(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }

        shift_vec(from, to, &mut self.bind_groups);
        self.runtime_revision.increase();
        self.project_revision.increase();
    }
}

impl ComputePassBindGroupEntry {
    pub fn new(bind_group_id: Option<BindGroupId>) -> Self {
        Self {
            id: fastrand::u64(..),
            bind_group_id,
        }
    }

    pub fn id(&self) -> ComputePassBindGroupEntryId {
        self.id
    }

    pub fn bind_group_id(&self) -> Option<BindGroupId> {
        self.bind_group_id
    }

    fn set_bind_group_id(&mut self, bind_group_id: Option<BindGroupId>) {
        self.bind_group_id = bind_group_id;
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

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_changed(id))
            || self
                .bind_groups
                .iter()
                .filter_map(|entry| entry.bind_group_id())
                .any(|id| tracker.was_changed(id))
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            ComputePassJob::Start => {
                if ctx.limits.max_compute_workgroups_per_dimension == 0 {
                    return Err(AppError::UnsupportedRendererFeature("Compute Passes"));
                }

                let mut bind_groups = vec![];
                for entry in &self.bind_groups {
                    let id = entry.bind_group_id().ok_or(AppError::UninitializedFields)?;
                    let Some(bind_group_runtime) = ctx.runtime_bind_groups.get_init(id)? else {
                        return Ok(SyncOutcome::Pending(ComputePassJob::Start));
                    };

                    bind_groups.push(bind_group_runtime);
                }

                let bind_group_layouts = bind_groups
                    .iter()
                    .map(|bg| Some(bg.inner_layout()))
                    .collect_vec();

                let pipeline_layout =
                    ctx.device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some(&format!("{} (Pipeline Layout)", self.label)),
                            bind_group_layouts: &bind_group_layouts,
                            immediate_size: 0,
                        });

                let shader_id = self.shader.ok_or(AppError::UninitializedFields)?;

                let Some(shader_runtime) = ctx.runtime_shaders.get_init(shader_id)? else {
                    return Ok(SyncOutcome::Pending(ComputePassJob::Start));
                };

                let scope = WgpuErrorScope::push(ctx.device);

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

                pass.dispatch_workgroups(
                    self.work_groups_x,
                    self.work_groups_y,
                    self.work_groups_z,
                );

                drop(pass);

                self.sync(ctx, None, ComputePassJob::Validation(scope.pop()))
            }
            ComputePassJob::Validation(mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(())),
                Poll::Pending => Ok(SyncOutcome::Pending(ComputePassJob::Validation(future))),
            },
        }
    }
}
