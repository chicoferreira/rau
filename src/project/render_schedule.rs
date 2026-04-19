use crate::{
    error::{AppError, AppResult},
    project::{
        Model, ProjectResource, RenderPassId, RenderScheduleId, Shader, TextureView,
        bindgroup::BindGroup,
        render_pass::{self, RenderPass},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, RuntimeCell, SyncOutcome, SyncResource, SyncTracker},
    },
};

#[derive(Default)]
pub struct RenderSchedule {
    entries: Vec<RenderPassId>,
}

pub struct RenderScheduleContext<'a> {
    pub device: &'a wgpu::Device,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub render_passes: &'a Storage<RenderPass>,
    pub runtime_render_passes: &'a RuntimeStorage<RenderPass>,
    pub models: &'a Storage<Model>,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

impl RenderSchedule {
    pub fn iter(&self) -> impl Iterator<Item = RenderPassId> {
        self.entries.iter().copied()
    }

    pub fn add(&mut self, render_pass_id: RenderPassId) {
        if self.iter().any(|entry| entry == render_pass_id) {
            return;
        }

        self.entries.push(render_pass_id);
    }
}

impl ProjectResource for RenderSchedule {
    type Id = RenderScheduleId;

    fn label(&self) -> &str {
        "Render Schedule"
    }
}

impl SyncResource for RenderSchedule {
    type Context<'a> = RenderScheduleContext<'a>;
    type Runtime = ();

    fn revision(&self) -> Revision {
        Revision::default()
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }

    fn should_sync(&self, _: &SyncTracker, _: &RuntimeCell<Self::Runtime>) -> bool {
        true
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

        for render_pass_id in self.iter() {
            let render_pass = ctx.render_passes.get(render_pass_id)?;
            let render_pass_runtime = ctx
                .runtime_render_passes
                .get(render_pass_id)?
                .ok_or(AppError::UninitResource)?;

            render_pass.submit(ctx.encoder, &render_ctx, render_pass_runtime)?;
        }

        Ok(SyncOutcome::Unchanged(()))
    }
}
