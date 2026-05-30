use crate::{
    error::{AppResult, RequiredFieldExt},
    project::{
        ProjectResource,
        resource::{
            bindgroup::BindGroup,
            model::Model,
            render_pass::RenderPass,
            render_pipeline::{RenderDrawStrategy, RenderPipeline},
            texture_view::TextureView,
        },
        storage::{RuntimeStorage, Storage},
    },
};

pub struct RenderContext<'a> {
    pub models: &'a Storage<Model>,
    pub render_pipelines: &'a Storage<RenderPipeline>,
    pub runtime_models: &'a RuntimeStorage<Model>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_render_pipelines: &'a RuntimeStorage<RenderPipeline>,
}

impl RenderPass {
    pub fn submit(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        render_ctx: &RenderContext<'_>,
    ) -> AppResult<()> {
        let color_target = self.target();
        let target_texture_id = color_target
            .texture_view_id()
            .ok_or_uninit_field("Color Target Texture")?;

        let Some(target_texture_view) = render_ctx
            .runtime_texture_views
            .get_init(target_texture_id)?
        else {
            return Ok(()); // Maybe return some kind of pending
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
                    return Ok(()); // Maybe return some kind of pending
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
                return Ok(()); // Maybe return some kind of pending
            };

            render_pass.set_pipeline(&pipeline_runtime.inner);

            for &(slot, id) in pipeline.static_bind_groups() {
                let Some(id) = id else {
                    continue;
                };
                let Some(bind_group) = render_ctx.runtime_bind_groups.get_init(id)? else {
                    return Ok(()); // Maybe return some kind of pending
                };
                render_pass.set_bind_group(slot, bind_group.inner(), &[]);
            }

            match pipeline.draw_strategy() {
                RenderDrawStrategy::Model {
                    model_id,
                    instances,
                    mesh_vertex_slot,
                    material_bind_group_slot,
                } => {
                    let model_id = model_id
                        .ok_or_uninit_field(format!("Pipeline {} Model Id", pipeline.label()))?;

                    let model = render_ctx.models.get(model_id)?;
                    let Some(model_runtime) = render_ctx.runtime_models.get_init(model_id)? else {
                        return Ok(()); // Maybe return some kind of pending
                    };

                    for (mesh_index, mesh) in model_runtime.meshes().iter().enumerate() {
                        let vertex_buffer = mesh.vertex_buffer().inner().slice(..);
                        render_pass.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer);

                        let index_buffer = mesh.index_buffer().inner().slice(..);
                        render_pass.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint32);

                        if let Some(mat_slot) = material_bind_group_slot {
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
                                return Ok(()); // Maybe return some kind of pending
                            };

                            render_pass.set_bind_group(*mat_slot, bind_group.inner(), &[]);
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

        Ok(())
    }
}
