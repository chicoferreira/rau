use crate::{model, project};

pub struct RenderPassSpecSet<'a> {
    pub render_passes: Vec<RenderPassSpec<'a>>,
}

impl RenderPassSpecSet<'_> {
    pub fn submit(&self, encoder: &mut wgpu::CommandEncoder, project: &project::Project) {
        for pass in &self.render_passes {
            pass.submit(encoder, project);
        }
    }
}

pub struct RenderPassSpec<'a> {
    pub label: Option<&'a str>,
    pub target_spec: RenderPassTargetSpec,
    pub depth_spec: Option<RenderPassDepthSpec<'a>>,
    pub pipelines: Vec<RenderPipelineSpec<'a>>,
}

impl RenderPassSpec<'_> {
    pub fn submit(&self, encoder: &mut wgpu::CommandEncoder, project: &project::Project) {
        let texture_entry = project
            .get_texture(self.target_spec.texture_id)
            .expect("deal with this later");

        let texture = texture_entry.texture();

        let view = match self.target_spec.texture_format {
            RenderPassTargetTextureFormat::UseExisting => &texture.view,
            RenderPassTargetTextureFormat::NewViewSrgb => {
                &texture.texture.create_view(&wgpu::TextureViewDescriptor {
                    format: Some(texture_entry.format().add_srgb_suffix()),
                    ..Default::default()
                })
            }
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
    }
}

pub struct RenderPassTargetSpec {
    pub texture_id: project::TextureId,
    pub texture_format: RenderPassTargetTextureFormat,
    pub load_operation: wgpu::LoadOp<wgpu::Color>,
}

pub enum RenderPassTargetTextureFormat {
    UseExisting,
    NewViewSrgb,
}

pub struct RenderPassDepthSpec<'a> {
    pub texture: &'a wgpu::TextureView, // TODO: change this to texture in a registry
    pub load_operation: wgpu::LoadOp<f32>,
}

pub struct RenderPipelineSpec<'a> {
    pub pipeline: &'a wgpu::RenderPipeline,
    pub vertex_buffers: Vec<RenderVertexBufferSpec<'a>>,
    pub bind_groups: Vec<RenderBindGroupSpec<'a>>,
    pub draw: RenderDrawSpec<'a>,
}

impl RenderPipelineSpec<'_> {
    pub fn draw(&self, render_pass: &mut wgpu::RenderPass, _project: &project::Project) {
        render_pass.set_pipeline(self.pipeline);

        match &self.draw {
            RenderDrawSpec::Model { model, instances } => {
                for mesh in &model.meshes {
                    for vertex_buffer in &self.vertex_buffers {
                        vertex_buffer.set(render_pass, Some(mesh));
                    }

                    for bind_group in &self.bind_groups {
                        bind_group.set(render_pass, Some((mesh, model)));
                    }

                    render_pass
                        .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.draw_indexed(0..mesh.num_elements, 0, instances.clone());
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
                    bind_group.set(render_pass, None);
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

    pub fn set(&self, render_pass: &mut wgpu::RenderPass, current_mesh: Option<&model::Mesh>) {
        match self {
            Self::ModelMesh { slot } => {
                let current_mesh = current_mesh.expect("deal with this later");
                render_pass.set_vertex_buffer(*slot, current_mesh.vertex_buffer.slice(..));
            }
            Self::Fixed { slot, buffer } => {
                render_pass.set_vertex_buffer(*slot, buffer.slice(..));
            }
        }
    }
}

pub struct RenderBindGroupSpec<'a> {
    slot: u32,
    target: RenderBindGroupTargetSpec<'a>,
}

impl<'a> RenderBindGroupSpec<'a> {
    pub fn new_fixed(slot: u32, target: &'a wgpu::BindGroup) -> Self {
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
        current: Option<(&model::Mesh, &model::Model)>,
    ) {
        match self.target {
            RenderBindGroupTargetSpec::Fixed(bind_group) => {
                render_pass.set_bind_group(self.slot, bind_group, &[]);
            }
            RenderBindGroupTargetSpec::ModelMaterial => {
                let (current_mesh, model) = current.expect("deal with this later");
                let material = &model.materials[current_mesh.material];
                render_pass.set_bind_group(self.slot, &material.bind_group, &[]);
            }
        }
    }
}

pub enum RenderBindGroupTargetSpec<'a> {
    Fixed(&'a wgpu::BindGroup),
    ModelMaterial,
}

pub enum RenderDrawSpec<'a> {
    Model {
        model: &'a model::Model,
        instances: std::ops::Range<u32>,
    },
    Single {
        vertices: std::ops::Range<u32>,
        instances: std::ops::Range<u32>,
    },
}
