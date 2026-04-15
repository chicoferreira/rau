use crate::{
    error::{AppError, AppResult},
    project::{BindGroupId, ModelId, Project, ProjectResource, ShaderId, TextureViewId},
};

pub struct RenderPass {
    pub label: String,
    pub target: RenderPassTarget<wgpu::Color>,
    pub depth_target: Option<RenderPassTarget<f32>>,
    pub pipelines: Vec<RenderPipeline>,
}

pub struct RenderPassTarget<T> {
    pub texture_view_id: TextureViewId,
    pub load_operation: wgpu::LoadOp<T>,
}

pub struct RenderPipeline {
    // wgpu RenderPipeline creation requires:
    // - List of bind groups (layouts)
    // - Texture Format to Render
    // - Depth Format (maybe fetch from the depth texture??)
    // - PrimitiveTopology (if it is a TriangleList, Points, etc)
    // - Shader
    // - Vertex Layout
    // wgpu RenderPass draw call requires:
    // - List of bind groups (to bind)
    // - The Index and Vertex buffer of the model
    pub label: String,
    pub primitive_state: wgpu::PrimitiveState,
    pub vertex_shader: ShaderId,
    pub fragment_shader: ShaderId,
    pub static_bind_groups: Vec<(u32, BindGroupId)>,
    pub draw: RenderDraw,
    inner: wgpu::RenderPipeline,
}

pub enum RenderDraw {
    Model {
        model_id: ModelId,
        instances: std::ops::Range<u32>,
        mesh_vertex_slot: u32,
        material_bind_group_slot: Option<u32>,
    },
    Direct {
        vertices: std::ops::Range<u32>,
        instances: std::ops::Range<u32>,
    },
}

impl ProjectResource for RenderPass {
    fn label(&self) -> &str {
        &self.label
    }
}

impl RenderPass {
    pub fn new(
        label: impl Into<String>,
        target: RenderPassTarget<wgpu::Color>,
        depth_target: Option<RenderPassTarget<f32>>,
    ) -> Self {
        let label = label.into();
        RenderPass {
            label,
            target,
            depth_target,
            pipelines: vec![],
        }
    }

    pub fn add_pipeline(
        &mut self,
        label: impl Into<String>,
        device: &wgpu::Device,
        project: &Project,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: ShaderId,
        fragment_shader: ShaderId,
        static_bind_groups: Vec<(u32, BindGroupId)>,
        draw: RenderDraw,
    ) -> AppResult<()> {
        let color_format = self.target.get_target_inner(project)?.texture().format();

        let depth_format = self
            .depth_target
            .as_ref()
            .map(|target| target.get_target_inner(project))
            .transpose()?
            .map(|view| view.texture().format());

        let pipeline = RenderPipeline::new(
            label.into(),
            device,
            project,
            color_format,
            depth_format,
            primitive_state,
            vertex_shader,
            fragment_shader,
            static_bind_groups,
            draw,
        )?;
        self.pipelines.push(pipeline);
        Ok(())
    }

    pub fn submit(&self, encoder: &mut wgpu::CommandEncoder, project: &Project) -> AppResult<()> {
        let color_view = self.target.get_target_inner(project)?;

        let depth_stencil_attachment = self
            .depth_target
            .as_ref()
            .map(|depth_target| -> AppResult<_> {
                let depth_view = depth_target.get_target_inner(project)?;
                Ok(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_target.load_operation,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            })
            .transpose()?;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&self.label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                ops: wgpu::Operations {
                    load: self.target.load_operation,
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

        for pipeline in &self.pipelines {
            pipeline.draw(&mut render_pass, project)?;
        }
        Ok(())
    }
}

impl RenderPipeline {
    fn new(
        label: String,
        device: &wgpu::Device,
        project: &Project,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: ShaderId,
        fragment_shader: ShaderId,
        static_bind_groups: Vec<(u32, BindGroupId)>,
        draw: RenderDraw,
    ) -> AppResult<Self> {
        let inner = Self::create_wgpu_pipeline(
            device,
            project,
            &label,
            &static_bind_groups,
            &draw,
            vertex_shader,
            fragment_shader,
            primitive_state,
            color_format,
            depth_format,
        )?;

        Ok(Self {
            label,
            primitive_state,
            vertex_shader,
            fragment_shader,
            static_bind_groups,
            draw,
            inner,
        })
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass, project: &Project) -> AppResult<()> {
        render_pass.set_pipeline(&self.inner);

        for &(slot, id) in &self.static_bind_groups {
            let bind_group = project.bind_groups.get(id)?;
            render_pass.set_bind_group(slot, bind_group.inner(), &[]);
        }

        match &self.draw {
            RenderDraw::Model {
                model_id,
                instances,
                mesh_vertex_slot,
                material_bind_group_slot,
            } => {
                let model = project.models.get(*model_id)?;

                for mesh in model.meshes() {
                    let vertex_buffer = mesh.vertex_buffer().inner();
                    render_pass.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer.slice(..));

                    if let Some(mat_slot) = material_bind_group_slot {
                        if let Some(material_index) = mesh.material_index() {
                            if let Some(material) = model.get_material(material_index) {
                                if let Some(bind_group_id) = material.bind_group_id() {
                                    let bind_group = project.bind_groups.get(bind_group_id)?;
                                    render_pass.set_bind_group(*mat_slot, bind_group.inner(), &[]);
                                }
                            }
                        }
                    }

                    let index_buffer = mesh.index_buffer().inner();
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

                    render_pass.draw_indexed(0..mesh.indices().len() as u32, 0, instances.clone());
                }
            }
            RenderDraw::Direct {
                vertices,
                instances,
                ..
            } => {
                render_pass.draw(vertices.clone(), instances.clone());
            }
        }
        Ok(())
    }

    fn create_wgpu_pipeline(
        device: &wgpu::Device,
        project: &Project,
        label: &str,
        static_bind_groups: &[(u32, BindGroupId)],
        draw: &RenderDraw,
        vertex_shader: ShaderId,
        fragment_shader: ShaderId,
        primitive_state: wgpu::PrimitiveState,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> AppResult<wgpu::RenderPipeline> {
        let bind_group_layouts =
            Self::resolved_bind_group_layout(project, &static_bind_groups, draw);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });

        let resolved_attributes_and_stride = Self::resolved_attributes_and_stride(project, draw)?;
        let vertex_buffers: &[wgpu::VertexBufferLayout] = match &resolved_attributes_and_stride {
            Some((attributes, array_stride)) => &[wgpu::VertexBufferLayout {
                array_stride: *array_stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes,
            }],
            None => &[],
        };

        let vertex_shader = project.shaders.get(vertex_shader)?;
        // TODO: move this shader creation to inside the shader struct for reutilization
        let vertex_shader = vertex_shader.create_wgpu_shader_module(device)?;

        let fragment_shader = project.shaders.get(fragment_shader)?;
        // TODO: move this shader creation to inside the shader struct for reutilization
        let fragment_shader = fragment_shader.create_wgpu_shader_module(device)?;

        Ok(
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader,
                    entry_point: None, // TODO: maybe allow for users to specify the entrypoint later?
                    buffers: vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: None, // TODO: maybe allow for users to specify the entrypoint later?
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: Some(wgpu::BlendState {
                            alpha: wgpu::BlendComponent::REPLACE,
                            color: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: primitive_state,
                depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                    format,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            }),
        )
    }

    fn resolved_bind_group_layout<'a>(
        project: &'a Project,
        static_bind_groups: &[(u32, BindGroupId)],
        draw: &RenderDraw,
    ) -> Vec<Option<&'a wgpu::BindGroupLayout>> {
        let layouts: Vec<(u32, &'a wgpu::BindGroupLayout)> = static_bind_groups
            .iter()
            .copied()
            .filter_map(|(slot, bind_group_id)| {
                let bind_group = project.bind_groups.get(bind_group_id).ok()?;
                Some((slot, bind_group.inner_layout()))
            })
            .chain(draw.material_bind_group_slot_and_layout(project))
            .collect();

        if layouts.is_empty() {
            return vec![];
        }

        let max_slot = layouts.iter().map(|(slot, _)| *slot).max().unwrap_or(0);
        let mut result = vec![None; max_slot as usize + 1];
        for (slot, layout) in layouts {
            result[slot as usize] = Some(layout);
        }
        result
    }

    fn resolved_attributes_and_stride(
        project: &Project,
        draw: &RenderDraw,
    ) -> AppResult<Option<(Vec<wgpu::VertexAttribute>, u64)>> {
        match &draw {
            RenderDraw::Model { model_id, .. } => {
                let model = project.models.get(*model_id)?;
                let spec = model.vertex_buffer_spec();
                Ok(Some(spec.to_wgpu_attributes_and_stride()))
            }
            RenderDraw::Direct { .. } => Ok(None),
        }
    }
}

impl<T> RenderPassTarget<T> {
    pub fn get_target_inner<'a>(&self, project: &'a Project) -> AppResult<&'a wgpu::TextureView> {
        let target_view = project.texture_views.get(self.texture_view_id)?;
        let inner = target_view.inner().as_ref();
        let inner = inner.ok_or(AppError::UninitResource(self.texture_view_id.into()))?;

        Ok(inner)
    }
}

impl RenderDraw {
    pub fn material_bind_group_slot_and_layout<'a>(
        &self,
        project: &'a Project,
    ) -> Option<(u32, &'a wgpu::BindGroupLayout)> {
        let RenderDraw::Model {
            model_id,
            material_bind_group_slot: Some(slot),
            ..
        } = self
        else {
            return None;
        };
        let model = project.models.get(*model_id).ok()?;
        let layout = model.get_bind_group_layout(project)?;
        Some((*slot, layout))
    }
}
