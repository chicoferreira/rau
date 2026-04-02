use crate::{
    error::AppResult,
    project::{
        self, BindGroupId,
        model::{Mesh, Model},
    },
};

pub struct RenderPassSpecSet<'a> {
    pub render_passes: Vec<RenderPassSpec<'a>>,
}

impl RenderPassSpecSet<'_> {
    pub fn submit(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        project: &project::Project,
    ) -> AppResult<()> {
        for pass in &self.render_passes {
            pass.submit(encoder, project)?;
        }
        Ok(())
    }
}

pub struct RenderPassSpec<'a> {
    pub label: Option<&'a str>,
    pub target_spec: RenderPassTargetSpec,
    pub depth_spec: Option<RenderPassDepthSpec<'a>>,
    pub pipelines: Vec<RenderPipelineSpec<'a>>,
}

impl RenderPassSpec<'_> {
    pub fn submit(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        project: &project::Project,
    ) -> AppResult<()> {
        let viewport_texture_view = project
            .texture_views
            .get(self.target_spec.texture_view_id)?
            .inner()
            .as_ref()
            .unwrap(); // TODO: FIX ME

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: viewport_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: self.target_spec.load_operation,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: self.depth_spec.as_ref().map(|spec| {
                wgpu::RenderPassDepthStencilAttachment {
                    view: &spec.texture,
                    depth_ops: Some(wgpu::Operations {
                        load: spec.load_operation,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        for pipeline in &self.pipelines {
            pipeline.draw(&mut render_pass, project);
        }
        Ok(())
    }
}

pub struct RenderPassTargetSpec {
    pub texture_view_id: project::TextureViewId,
    pub load_operation: wgpu::LoadOp<wgpu::Color>,
}

pub struct RenderPassDepthSpec<'a> {
    pub texture: &'a wgpu::TextureView, // TODO: change this to viewport in a registry
    pub load_operation: wgpu::LoadOp<f32>,
}

pub struct RenderPipelineSpec<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub vertex_buffers: Vec<RenderVertexBufferSpec<'a>>,
    pub bind_groups: Vec<RenderBindGroupSpec>,
    pub draw: RenderDrawSpec<'a>,
}

impl RenderPipelineSpec<'_> {
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass, project: &project::Project) {
        render_pass.set_pipeline(self.pipeline);

        match &self.draw {
            RenderDrawSpec::Model { model, instances } => {
                for mesh in model.meshes() {
                    for vertex_buffer in &self.vertex_buffers {
                        vertex_buffer.set(render_pass, Some(mesh));
                    }

                    for bind_group in &self.bind_groups {
                        bind_group.set(render_pass, Some(mesh), Some(model), project);
                    }

                    let index_buffer = mesh.index_buffer().inner();
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.draw_indexed(0..mesh.indices().len() as u32, 0, instances.clone());
                }
            }
            RenderDrawSpec::Single {
                vertices,
                instances,
            } => {
                for vertex_buffer in &self.vertex_buffers {
                    vertex_buffer.set(render_pass, None);
                }

                for bind_group in &self.bind_groups {
                    bind_group.set(render_pass, None, None, project);
                }

                render_pass.draw(vertices.clone(), instances.clone());
            }
        }
    }
}

pub enum RenderVertexBufferSpec<'a> {
    ModelMesh { slot: u32 },
    Fixed { slot: u32, buffer: &'a wgpu::Buffer },
}

impl<'a> RenderVertexBufferSpec<'a> {
    pub fn new_model_mesh(slot: u32) -> Self {
        Self::ModelMesh { slot }
    }

    pub fn new_fixed(slot: u32, buffer: &'a wgpu::Buffer) -> Self {
        Self::Fixed { slot, buffer }
    }

    pub fn set(&self, render_pass: &mut wgpu::RenderPass, current_mesh: Option<&Mesh>) {
        match self {
            Self::ModelMesh { slot } => {
                let current_mesh = current_mesh.expect("deal with this later");
                let vertex_buffer = current_mesh.vertex_buffer().inner();
                render_pass.set_vertex_buffer(*slot, vertex_buffer.slice(..));
            }
            Self::Fixed { slot, buffer } => {
                render_pass.set_vertex_buffer(*slot, buffer.slice(..));
            }
        }
    }
}

pub struct RenderBindGroupSpec {
    slot: u32,
    target: RenderBindGroupTargetSpec,
}

impl RenderBindGroupSpec {
    pub fn new_fixed(slot: u32, target: BindGroupId) -> Self {
        Self {
            slot,
            target: RenderBindGroupTargetSpec::Fixed(target),
        }
    }

    pub fn new_model_material(slot: u32) -> Self {
        Self {
            slot,
            target: RenderBindGroupTargetSpec::ModelMaterial,
        }
    }

    pub fn set(
        &self,
        render_pass: &mut wgpu::RenderPass,
        current_mesh: Option<&Mesh>,
        current_model: Option<&Model>,
        project: &project::Project,
    ) {
        match self.target {
            RenderBindGroupTargetSpec::Fixed(bind_group_id) => {
                let bind_group = project
                    .bind_groups
                    .get(bind_group_id)
                    .expect("deal with this later");
                render_pass.set_bind_group(self.slot, bind_group.inner(), &[]);
            }
            RenderBindGroupTargetSpec::ModelMaterial => {
                let current_mesh = current_mesh.expect("deal with this later");
                let current_model = current_model.expect("deal with this later");

                if let Some(material_index) = current_mesh.material_index() {
                    if let Some(material) = current_model.get_material(material_index) {
                        if let Some(bind_group_id) = material.bind_group_id() {
                            let bind_group = project
                                .bind_groups
                                .get(bind_group_id)
                                .expect("deal with this later");
                            render_pass.set_bind_group(self.slot, bind_group.inner(), &[]);
                        }
                    }
                }
            }
        }
    }
}

pub enum RenderBindGroupTargetSpec {
    Fixed(BindGroupId),
    ModelMaterial,
}

pub enum RenderDrawSpec<'a> {
    Model {
        model: &'a Model,
        instances: std::ops::Range<u32>,
    },
    Single {
        vertices: std::ops::Range<u32>,
        instances: std::ops::Range<u32>,
    },
}
