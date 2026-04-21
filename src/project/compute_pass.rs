use itertools::Itertools;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, ComputePassId, ProjectResource, ShaderId,
        bindgroup::BindGroup,
        shader::Shader,
        storage::RuntimeStorage,
        sync::{Revision, SyncOutcome, SyncResource},
    },
};

pub struct ComputePass {
    pub label: String,
    pub bind_groups: Vec<BindGroupId>,
    pub shader: Option<ShaderId>,
    pub work_groups_x: u32,
    pub work_groups_y: u32,
    pub work_groups_z: u32,
    revision: Revision,
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

impl ComputePass {
    pub fn new(
        label: impl Into<String>,
        bind_groups: Vec<BindGroupId>,
        shader: Option<ShaderId>,
        work_groups_x: u32,
        work_groups_y: u32,
        work_groups_z: u32,
    ) -> Self {
        Self {
            label: label.into(),
            bind_groups,
            shader,
            work_groups_x,
            work_groups_y,
            work_groups_z,
            revision: Revision::default(),
        }
    }
}

impl ProjectResource for ComputePass {
    type Id = ComputePassId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for ComputePass {
    type Context<'a> = Context<'a>;
    type Runtime = ();

    fn revision(&self) -> super::sync::Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &super::sync::SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_changed(id))
            || self.bind_groups.iter().any(|id| tracker.was_changed(*id))
    }

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let mut bind_groups = vec![];
        for id in &self.bind_groups {
            let bind_group_runtime = ctx
                .runtime_bind_groups
                .get(*id)
                .and_then(|r| r.ok_or(AppError::UninitResource))?;

            bind_groups.push(bind_group_runtime);
        }

        let bind_group_layouts = bind_groups
            .iter()
            .map(|bg| Some(bg.inner_layout()))
            .collect_vec();

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{} (Pipeline Layout)", self.label)),
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            });

        let Some(shader_id) = self.shader else {
            return Err(AppError::UninitResource);
        };

        let shader_runtime = ctx
            .runtime_shaders
            .get(shader_id)
            .and_then(|r| r.ok_or(AppError::UninitResource))?;

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

        pass.dispatch_workgroups(self.work_groups_x, self.work_groups_y, self.work_groups_z);

        Ok(SyncOutcome::Changed(()))
    }
}
