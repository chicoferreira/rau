use std::task::Poll;

use slotmap::SecondaryMap;
use wgpu_profiler::GpuProfiler;

use crate::{
    error::{AppError, AppResult},
    project::{
        ComputePassId, ProjectResource, ProjectRevisionSnapshot, RuntimeProject,
        resource::{
            bindgroup::BindGroup,
            compute_pass::{ComputePass, DispatchPolicy},
            dimension::Dimension,
            presentation::Presentation,
            render_pass::{RenderPass, RenderPassRuntime},
            texture_view::TextureView,
        },
        storage::{RuntimeStorage, Storage},
        sync::SyncTracker,
    },
    utils::async_job::AsyncJob,
};

pub struct RenderContext<'a> {
    pub render_passes: &'a Storage<RenderPass>,
    pub runtime_render_passes: &'a RuntimeStorage<RenderPass>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub gpu_profiler: &'a GpuProfiler,
}

pub struct ComputeDispatchContext<'a> {
    pub compute_passes: &'a Storage<ComputePass>,
    pub runtime_compute_passes: &'a mut RuntimeStorage<ComputePass>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub dimensions: &'a Storage<Dimension>,
    pub compute_accumulators: &'a mut SecondaryMap<ComputePassId, instant::Duration>,
    pub tracker: &'a SyncTracker,
    pub gpu_profiler: &'a GpuProfiler,
    pub dt: instant::Duration,
}

#[derive(Default)]
pub enum PresentationRender {
    #[default]
    Idle,
    Pending {
        job: AsyncJob<AppResult<()>>,
        snapshot: ProjectRevisionSnapshot,
    },
    Errored {
        error: AppError,
        snapshot: ProjectRevisionSnapshot,
    },
}

impl PresentationRender {
    pub fn error(&self) -> Option<&AppError> {
        match self {
            PresentationRender::Errored { error, .. } => Some(error),
            PresentationRender::Idle | PresentationRender::Pending { .. } => None,
        }
    }
}

impl RuntimeProject {
    pub fn poll_presentation_errors(
        &mut self,
        current_snapshot: ProjectRevisionSnapshot,
        runtime_resources_changed: bool,
    ) -> bool {
        if let PresentationRender::Pending { job, snapshot } = &mut self.presentation_render {
            match job.try_resolve() {
                Poll::Ready(Ok(())) => {
                    self.presentation_render = PresentationRender::Idle;
                }
                Poll::Ready(Err(error)) => {
                    let snapshot = std::mem::take(snapshot);
                    self.presentation_render = PresentationRender::Errored { error, snapshot };
                }
                Poll::Pending => {}
            }
        }

        if let PresentationRender::Errored { snapshot, .. } = &self.presentation_render {
            if current_snapshot == *snapshot && !runtime_resources_changed {
                return false; // Shouldn't render the frame because it is still errored and nothing has changed
            }
            // A resource changed since the error: clear it and try rendering again.
            self.presentation_render = PresentationRender::Idle;
        }

        true // Should render the frame
    }

    /// Records a submitted frame's error scope for [`Self::poll_presentation_errors`].
    ///
    /// Only one submission is tracked: submitting while still `Pending` drops the
    /// new scope unmonitored. Scopes settle by the next frame (see
    /// [`WgpuErrorScope`](crate::utils::wgpu_error_scope::WgpuErrorScope)), so no
    /// queue of outstanding frames needs to be tracked.
    pub fn on_frame_submitted(
        &mut self,
        current_snapshot: ProjectRevisionSnapshot,
        job: AsyncJob<AppResult<()>>,
    ) {
        if let PresentationRender::Idle = self.presentation_render {
            let snapshot = current_snapshot;
            self.presentation_render = PresentationRender::Pending { job, snapshot };
        }
    }
}

impl Presentation {
    /// Dispatches the scheduled compute passes in order, into `encoder`, deciding
    /// per pass whether it runs this frame from its [`DispatchPolicy`].
    ///
    /// The build step (pipeline creation in [`ComputePass`]'s `sync`) is separate;
    /// this only emits the dispatches.
    ///
    /// Returns whether at least one dispatch was encoded into `encoder`.
    pub fn dispatch_computes(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ctx: &mut ComputeDispatchContext<'_>,
    ) -> bool {
        let mut encoded_any = false;

        for compute_pass_id in self.compute_passes() {
            let id = *compute_pass_id;

            let Ok(compute_pass) = ctx.compute_passes.get(id) else {
                continue; // dangling id in the schedule
            };

            let runtime = match ctx.runtime_compute_passes.get_init(id) {
                Ok(Some(runtime)) => runtime,
                Ok(None) | Err(_) => continue,
            };

            let should_dispatch = match compute_pass.dispatch_policy() {
                DispatchPolicy::EveryFrame => true,
                // `was_recreated` covers the frame the pipeline was first built or
                // rebuilt; `inputs_changed` covers data-only changes to its inputs.
                DispatchPolicy::OnChange => {
                    ctx.tracker.was_recreated(id) || compute_pass.inputs_changed(ctx.tracker)
                }
                DispatchPolicy::Periodic { interval } => {
                    let accumulated = ctx
                        .compute_accumulators
                        .entry(id)
                        .expect("compute pass id is valid")
                        .or_insert(instant::Duration::ZERO);
                    *accumulated += ctx.dt;
                    if *accumulated >= interval {
                        // Subtract one interval (rather than reset to zero) to keep
                        // the average cadence accurate. Clamp so a long stall can't
                        // build up a backlog of catch-up dispatches.
                        *accumulated = (*accumulated - interval).min(interval);
                        true
                    } else {
                        false
                    }
                }
            };

            if should_dispatch {
                // A wgpu validation error during encoding is caught by the frame-wide
                // error scope in the app's render loop. An `Err` here is a Rust-side
                // failure (e.g. a bound resource has errored); record it on the pass's
                // own runtime cell so it surfaces like any other resource error. The
                // error state only changes on an actual dispatch (or a rebuild), not
                // every frame, since dispatches don't happen every frame.
                let encode = compute_pass.encode(
                    encoder,
                    ctx.gpu_profiler,
                    runtime,
                    ctx.runtime_bind_groups,
                    ctx.dimensions,
                );
                match encode {
                    Ok(encoded) => encoded_any |= encoded,
                    Err(error) => ctx.runtime_compute_passes.mark_errored(id, error),
                }
            }
        }

        encoded_any
    }

    /// Begins every render pass and replays its recorded bundle into `encoder`.
    ///
    /// The draws themselves live in each pass's [`RenderPassRuntime`], recorded once
    /// during the sync step.
    ///
    /// Returns `Ok(false)` as soon as a pass bails out, either because one of its target
    /// texture views or its own bundle is still pending, or because recording that
    /// bundle failed. `Err` is only returned for presentation-level problems, such
    /// as a render pass id that no longer resolves to a resource.
    ///
    /// The caller should drop the encoder without finishing it whenever this returns `Ok(false)`,
    /// so the half-encoded passes never reach the GPU and the viewport keeps the previous frame
    /// instead of flickering the clear color.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_ctx: &RenderContext<'_>,
    ) -> AppResult<bool> {
        for render_pass_id in self.render_passes() {
            let render_pass = render_ctx.render_passes.get(*render_pass_id)?;

            let runtime = match render_ctx.runtime_render_passes.get_init(*render_pass_id) {
                Ok(Some(runtime)) => runtime,
                // the bundle is either still being recorded or failed to record,
                // so avoid rendering this pass.
                Ok(None) | Err(_) => return Ok(false),
            };

            if !render_pass.execute(
                encoder,
                render_ctx.gpu_profiler,
                runtime,
                render_ctx.runtime_texture_views,
            )? {
                return Ok(false); // a target texture view is still pending
            }
        }

        Ok(true)
    }
}

impl RenderPass {
    /// Begins this pass on `encoder` and executes its recorded bundle in it.
    ///
    /// Returns `Ok(true)` once the pass is encoded, or `Ok(false)` if it bailed out
    /// because a target texture view is still pending.
    pub fn execute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        gpu_profiler: &GpuProfiler,
        runtime: &RenderPassRuntime,
        runtime_texture_views: &RuntimeStorage<TextureView>,
    ) -> AppResult<bool> {
        let Some(color_attachments) =
            self.map_color_targets(runtime_texture_views, |target, view| {
                wgpu::RenderPassColorAttachment {
                    view: view.inner(),
                    ops: wgpu::Operations {
                        load: target.load_operation().into(),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                    resolve_target: None,
                }
            })?
        else {
            return Ok(false); // pending: a target texture view is not ready
        };

        let Some(depth_stencil_attachment) =
            self.map_depth_target(runtime_texture_views, |target, view| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: view.inner(),
                    depth_ops: Some(wgpu::Operations {
                        load: target.load_operation().into(),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            })?
        else {
            return Ok(false); // pending: depth texture view not ready
        };

        let query = gpu_profiler.begin_pass_query(self.label(), encoder);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(self.label()),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            occlusion_query_set: None,
            timestamp_writes: query.render_pass_timestamp_writes(),
            multiview_mask: None,
        });

        render_pass.execute_bundles([runtime.bundle()]);
        drop(render_pass);

        gpu_profiler.end_query(encoder, query);

        Ok(true)
    }
}
