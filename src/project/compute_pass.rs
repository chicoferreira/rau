use egui_dnd::utils::shift_vec;
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
    label: String,
    bind_groups: Vec<ComputePassBindGroupEntry>,
    shader: Option<ShaderId>,
    work_groups_x: u32,
    work_groups_y: u32,
    work_groups_z: u32,
    revision: Revision,
}

pub type ComputePassBindGroupEntryId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComputePassBindGroupEntry {
    id: ComputePassBindGroupEntryId,
    bind_group_id: Option<BindGroupId>,
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
            revision: Revision::default(),
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
            self.revision.increase();
        }
    }

    pub fn set_shader(&mut self, shader: Option<ShaderId>) {
        if self.shader != shader {
            self.shader = shader;
            self.revision.increase();
        }
    }

    pub fn set_work_groups(&mut self, x: u32, y: u32, z: u32) {
        let next = (x.max(1), y.max(1), z.max(1));
        let current = (self.work_groups_x, self.work_groups_y, self.work_groups_z);
        if current != next {
            self.work_groups_x = next.0;
            self.work_groups_y = next.1;
            self.work_groups_z = next.2;
            self.revision.increase();
        }
    }

    pub fn set_bind_group(&mut self, index: usize, bind_group: Option<BindGroupId>) {
        if let Some(current) = self.bind_groups.get_mut(index)
            && current.bind_group_id() != bind_group
        {
            current.set_bind_group_id(bind_group);
            self.revision.increase();
        }
    }

    pub fn add_bind_group(&mut self, bind_group: Option<BindGroupId>) {
        self.bind_groups
            .push(ComputePassBindGroupEntry::new(bind_group));
        self.revision.increase();
    }

    pub fn remove_bind_group(&mut self, index: usize) {
        if index < self.bind_groups.len() {
            self.bind_groups.remove(index);
            self.revision.increase();
        }
    }

    pub fn reorder_bind_groups(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }

        shift_vec(from, to, &mut self.bind_groups);
        self.revision.increase();
    }
}

impl ComputePassBindGroupEntry {
    pub fn new(bind_group_id: Option<BindGroupId>) -> Self {
        Self {
            id: fastrand::usize(..),
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
}

impl SyncResource for ComputePass {
    type Context<'a> = Context<'a>;
    type Runtime = ();

    fn revision(&self) -> super::sync::Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &super::sync::SyncTracker) -> bool {
        self.shader.is_some_and(|id| tracker.was_changed(id))
            || self
                .bind_groups
                .iter()
                .filter_map(|entry| entry.bind_group_id())
                .any(|id| tracker.was_changed(id))
    }

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let mut bind_groups = vec![];
        for entry in &self.bind_groups {
            let id = entry.bind_group_id().ok_or(AppError::UninitResource)?;
            let bind_group_runtime = ctx
                .runtime_bind_groups
                .get(id)
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
