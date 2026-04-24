use egui_dnd::utils::shift_vec;

use crate::{
    error::{AppError, AppResult},
    project::{
        FramePlanId, Model, ProjectResource, RenderPassId, Shader, TextureView,
        bindgroup::BindGroup,
        render_pass::{self, RenderPass},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, RuntimeCell, SyncOutcome, SyncResource, SyncTracker},
    },
};

#[derive(Default)]
pub struct FramePlan {
    entries: Vec<FramePlanStep>,
    revision: Revision,
}

pub type FramePlanStepId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePlanStep {
    id: FramePlanStepId,
    render_pass_id: Option<RenderPassId>,
}

pub struct FramePlanContext<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub render_passes: &'a Storage<RenderPass>,
    pub runtime_render_passes: &'a RuntimeStorage<RenderPass>,
    pub models: &'a Storage<Model>,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

impl FramePlan {
    pub fn entries(&self) -> &[FramePlanStep] {
        &self.entries
    }

    pub fn add(&mut self, render_pass_id: Option<RenderPassId>) {
        self.entries.push(FramePlanStep::new(render_pass_id));
        self.revision.increase();
    }

    pub fn update_entry(&mut self, index: usize, render_pass_id: Option<RenderPassId>) {
        if let Some(entry) = self.entries.get_mut(index)
            && entry.render_pass_id != render_pass_id
        {
            entry.render_pass_id = render_pass_id;
            self.revision.increase();
        }
    }

    pub fn remove_entry(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
            self.revision.increase();
        }
    }

    pub fn reorder_entries(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }

        shift_vec(from, to, &mut self.entries);
        self.revision.increase();
    }
}

impl ProjectResource for FramePlan {
    type Id = FramePlanId;

    fn label(&self) -> &str {
        "Frame Plan"
    }
}

impl FramePlanStep {
    fn new(render_pass_id: Option<RenderPassId>) -> Self {
        Self {
            id: fastrand::usize(..),
            render_pass_id,
        }
    }

    pub fn id(&self) -> FramePlanStepId {
        self.id
    }

    pub fn render_pass_id(&self) -> Option<RenderPassId> {
        self.render_pass_id
    }
}

impl SyncResource for FramePlan {
    type Context<'a> = FramePlanContext<'a>;
    type Runtime = ();

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }

    fn should_sync(&self, tracker: &SyncTracker, runtime: &RuntimeCell<Self::Runtime>) -> bool {
        match runtime {
            RuntimeCell::Empty | RuntimeCell::Created { .. } => true,
            RuntimeCell::Errored {
                revision: at_revision,
                ..
            }
            | RuntimeCell::PendingValidation {
                revision: at_revision,
                ..
            } => *at_revision != self.revision || tracker.has_changes(),
        }
    }

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let render_ctx = render_pass::Context {
            device: ctx.device,
            models: ctx.models,
            runtime_shaders: ctx.runtime_shaders,
            runtime_texture_views: ctx.runtime_texture_views,
            runtime_bind_groups: ctx.runtime_bind_groups,
        };

        for entry in self.entries() {
            let render_pass_id = entry.render_pass_id.ok_or(AppError::UninitializedFields)?;
            let render_pass = ctx.render_passes.get(render_pass_id)?;
            let render_pass_runtime = ctx.runtime_render_passes.get_init(render_pass_id)?;

            render_pass.submit(ctx.encoder, &render_ctx, render_pass_runtime)?;
        }

        Ok(SyncOutcome::Unchanged(()))
    }
}
