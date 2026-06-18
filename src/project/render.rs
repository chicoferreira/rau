use std::task::Poll;

use slotmap::SecondaryMap;

use crate::{
    error::{AppError, AppResult, RequiredFieldExt},
    project::{
        ProjectResource, ProjectRevisionSnapshot, RenderPassId, RuntimeProject,
        resource::{
            bindgroup::BindGroup,
            model::Model,
            presentation::Presentation,
            render_pass::RenderPass,
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            texture_view::TextureView,
        },
        storage::{RuntimeStorage, Storage},
    },
    utils::async_job::AsyncJob,
};

pub struct RenderContext<'a> {
    pub models: &'a Storage<Model>,
    pub render_pipelines: &'a Storage<RenderPipeline>,
    pub render_passes: &'a Storage<RenderPass>,
    pub runtime_models: &'a RuntimeStorage<Model>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_render_pipelines: &'a RuntimeStorage<RenderPipeline>,
    pub render_pass_errors: &'a mut SecondaryMap<RenderPassId, AppError>,
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
    /// Encodes every render pass into `encoder`, recording any per-pass errors in
    /// [`RenderContext::render_pass_errors`].
    ///
    /// Returns `Ok(false)` as soon as a pass bails out, either because one of its runtime resources
    /// is still pending or because encoding it failed (the error is then recorded on that pass).
    /// `Err` is only returned for presentation-level problems, such as a render pass id that no
    /// longer resolves to a resource.
    ///
    /// The caller should drop the encoder without finishing it whenever this returns `Ok(false)`,
    /// so the half-encoded passes never reach the GPU and the viewport keeps the previous frame
    /// instead of flickering the clear color.
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_ctx: &mut RenderContext<'_>,
    ) -> AppResult<bool> {
        render_ctx.render_pass_errors.clear();

        for render_pass_id in self.render_passes() {
            let render_pass = render_ctx.render_passes.get(*render_pass_id)?;

            match render_pass.submit(encoder, render_ctx) {
                Ok(true) => {}                 // fully encoded
                Ok(false) => return Ok(false), // a runtime resource is still pending
                Err(error) => {
                    // An encoding error (a missing target, an errored model, ...) is attributed to
                    // the pass itself rather than bubbled up to the presentation, so it is easier to
                    // trace. We then bail so the half-encoded frame is dropped by the caller.
                    render_ctx.render_pass_errors.insert(*render_pass_id, error);
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}

impl RenderPass {
    /// Encodes this render pass into `encoder`.
    ///
    /// Returns `Ok(true)` once the pass is fully encoded, or `Ok(false)` if it bailed
    /// out because a runtime resource (texture view, pipeline, bind group, model) is
    /// still pending.
    pub fn submit(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_ctx: &RenderContext<'_>,
    ) -> AppResult<bool> {
        let color_target = self.target();
        let target_texture_id = color_target
            .texture_view_id()
            .ok_or_uninit_field("Color Target Texture")?;

        let Some(target_texture_view) = render_ctx
            .runtime_texture_views
            .get_init(target_texture_id)?
        else {
            return Ok(false); // pending: target texture view not ready
        };

        let view = target_texture_view.inner();

        let depth_stencil_attachment = match self.depth_target() {
            Some(depth_target) => {
                // TODO: solve this duplicated code from above
                let depth_texture_id = depth_target
                    .texture_view_id()
                    .ok_or_uninit_field("Depth Target Texture")?;

                let Some(depth_texture_view) = render_ctx
                    .runtime_texture_views
                    .get_init(depth_texture_id)?
                else {
                    return Ok(false); // pending: depth texture view not ready
                };

                Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_texture_view.inner(),
                    depth_ops: Some(wgpu::Operations {
                        load: depth_target.load_operation().into(),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            }
            None => None,
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(self.label()),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                ops: wgpu::Operations {
                    load: color_target.load_operation().into(),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
                resolve_target: None,
            })],
            depth_stencil_attachment,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        for id in self.pipelines() {
            let pipeline = render_ctx.render_pipelines.get(*id)?;
            let Some(pipeline_runtime) = render_ctx.runtime_render_pipelines.get_init(*id)? else {
                return Ok(false); // pending: pipeline still rebuilding
            };

            render_pass.set_pipeline(&pipeline_runtime.inner);

            let mut material_bind_group_slots = vec![];
            for (slot, bind_group_target) in pipeline.bind_groups().into_iter().enumerate() {
                let slot = slot as u32;
                match bind_group_target {
                    BindGroupTarget::Empty => {
                        render_pass.set_bind_group(slot, None, &[]);
                    }
                    BindGroupTarget::Static(id) => {
                        let Some(bind_group) = render_ctx.runtime_bind_groups.get_init(*id)? else {
                            return Ok(false); // pending: static bind group not ready
                        };
                        render_pass.set_bind_group(slot, bind_group.inner(), &[]);
                    }
                    BindGroupTarget::ModelMaterial => {
                        material_bind_group_slots.push(slot);
                    }
                }
            }

            match pipeline.draw_strategy() {
                RenderDrawStrategy::Model {
                    model_id,
                    instances,
                    mesh_vertex_slot,
                } => {
                    let model_id = model_id
                        .ok_or_uninit_field(format!("Pipeline {} Model Id", pipeline.label()))?;

                    let model = render_ctx.models.get(model_id)?;
                    let Some(model_runtime) = render_ctx.runtime_models.get_init(model_id)? else {
                        return Ok(false); // pending: model not ready
                    };

                    for (mesh_index, mesh) in model_runtime.meshes().iter().enumerate() {
                        let vertex_buffer = mesh.vertex_buffer().inner().slice(..);
                        render_pass.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer);

                        let index_buffer = mesh.index_buffer().inner().slice(..);
                        render_pass.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint32);

                        if !material_bind_group_slots.is_empty() {
                            let material_index = model
                                .selected_material_index(mesh_index, mesh)
                                .ok_or_uninit_field(format!(
                                    "Pipeline {} Model {} Mesh {mesh_index} Selected Material",
                                    pipeline.label(),
                                    model.label(),
                                ))?;
                            // TODO: Maybe this should be changed to a chain of `ok_or_uninit_field` calls?

                            let bind_group_id = model
                                .material_bind_group_id(material_index)
                                .ok_or_uninit_field(format!(
                                    "Pipeline {} Model {} Mesh {mesh_index} Material {material_index} Bind Group Id",
                                    pipeline.label(),
                                    model.label(),
                                ))?;

                            let Some(bind_group) =
                                render_ctx.runtime_bind_groups.get_init(bind_group_id)?
                            else {
                                return Ok(false); // pending: material bind group not ready
                            };

                            for slot in &material_bind_group_slots {
                                render_pass.set_bind_group(*slot, bind_group.inner(), &[]);
                            }
                        }

                        let index_num = mesh.indices().len() as u32;
                        render_pass.draw_indexed(0..index_num, 0, instances.clone());
                    }
                }
                RenderDrawStrategy::Direct {
                    vertices,
                    instances,
                } => render_pass.draw(vertices.clone(), instances.clone()),
            }
        }

        Ok(true)
    }
}
