use egui_dnd::utils::shift_vec;

use crate::{
    error::{AppError, AppResult, WgpuErrorScope},
    project::{
        BindGroupId, ModelId, ProjectResource, RenderPassId, ShaderId, TextureViewId,
        bindgroup::BindGroup,
        model::Model,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
        shader::Shader,
        storage::Storage,
        texture_view::TextureView,
    },
};

pub struct RenderPass {
    pub label: String,
    pub target: RenderPassTarget<Color>,
    pub depth_target: Option<RenderPassTarget<f32>>,
    pub pipelines: Vec<RenderPipeline>,
    dirty: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct RenderPassTarget<T> {
    pub texture_view_id: Option<TextureViewId>,
    pub load_operation: LoadOperation<T>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum LoadOperation<T> {
    Clear(T),
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub [f32; 4]);

pub type RenderPipelineId = usize;

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
    /// Used for stability in pipeline reordering
    pub id: RenderPipelineId,
    pub label: String,
    pub primitive_state: wgpu::PrimitiveState,
    pub vertex_shader: Option<ShaderId>,
    pub fragment_shader: Option<ShaderId>,
    pub static_bind_groups: Vec<(u32, BindGroupId)>,
    pub draw: RenderDraw,
    inner: Option<wgpu::RenderPipeline>,
    dirty: bool,
    has_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RenderDraw {
    Model {
        model_id: Option<ModelId>,
        instances: std::ops::Range<u32>,
        mesh_vertex_slot: u32,
        material_bind_group_slot: Option<u32>,
    },
    Direct {
        vertices: std::ops::Range<u32>,
        instances: std::ops::Range<u32>,
    },
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub models: &'a Storage<ModelId, Model>,
    pub shaders: &'a Storage<ShaderId, Shader>,
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
    pub bind_groups: &'a Storage<BindGroupId, BindGroup>,
}

impl ProjectResource for RenderPass {
    fn label(&self) -> &str {
        &self.label
    }
}

impl RenderPass {
    pub fn set_target(&mut self, target: RenderPassTarget<Color>) {
        self.target = target;
        self.dirty = true;
    }

    pub fn set_depth_target(&mut self, target: Option<RenderPassTarget<f32>>) {
        self.depth_target = target;
        self.dirty = true;
    }

    pub fn new(
        label: impl Into<String>,
        target: RenderPassTarget<Color>,
        depth_target: Option<RenderPassTarget<f32>>,
    ) -> Self {
        let label = label.into();
        RenderPass {
            label,
            target,
            depth_target,
            pipelines: vec![],
            dirty: false,
        }
    }

    pub fn add_pipeline(
        &mut self,
        label: impl Into<String>,
        ctx: &Context,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: Option<ShaderId>,
        fragment_shader: Option<ShaderId>,
        static_bind_groups: Vec<(u32, BindGroupId)>,
        draw: RenderDraw,
    ) -> AppResult<()> {
        let color_format = self.get_color_format(ctx)?;
        let depth_format = self.get_depth_format(ctx)?;

        let pipeline = RenderPipeline::new(
            label.into(),
            ctx,
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

    pub fn add_empty_pipeline(&mut self, label: impl Into<String>) {
        let pipeline = RenderPipeline::empty(label.into());
        self.pipelines.push(pipeline);
    }

    pub fn remove_pipeline(&mut self, index: usize) {
        if index < self.pipelines.len() {
            self.pipelines.remove(index);
        }
    }

    pub fn reorder_pipelines(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.pipelines);
    }

    pub fn get_color_format(&self, ctx: &Context) -> AppResult<wgpu::TextureFormat> {
        let target_view = self.target.get_target_inner(ctx.texture_views)?;
        Ok(target_view.texture().format())
    }

    pub fn get_depth_format(&self, ctx: &Context) -> AppResult<Option<wgpu::TextureFormat>> {
        Ok(self
            .depth_target
            .as_ref()
            .map(|target| target.get_target_inner(ctx.texture_views))
            .transpose()?
            .map(|view| view.texture().format()))
    }

    pub fn submit(&self, encoder: &mut wgpu::CommandEncoder, ctx: &Context) -> AppResult<()> {
        let color_view = self.target.get_target_inner(ctx.texture_views)?;

        let depth_stencil_attachment = self
            .depth_target
            .as_ref()
            .map(|depth_target| -> AppResult<_> {
                let depth_view = depth_target.get_target_inner(ctx.texture_views)?;
                Ok(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: depth_target.load_operation.into(),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            })
            .transpose()?;

        let scope = WgpuErrorScope::push(ctx.device);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&self.label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color_view,
                ops: wgpu::Operations {
                    load: self.target.load_operation.into(),
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
            pipeline.draw(&mut render_pass, ctx)?;
        }
        scope.pop()?;

        Ok(())
    }
}

impl RenderPipeline {
    pub fn set_vertex_shader(&mut self, id: Option<ShaderId>) {
        self.vertex_shader = id;
        self.dirty = true;
    }

    pub fn set_fragment_shader(&mut self, id: Option<ShaderId>) {
        self.fragment_shader = id;
        self.dirty = true;
    }

    pub fn set_primitive_state(&mut self, primitive_state: wgpu::PrimitiveState) {
        self.primitive_state = primitive_state;
        self.dirty = true;
    }

    pub fn set_draw(&mut self, draw: RenderDraw) {
        self.draw = draw;
        self.dirty = true;
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.dirty = true;
    }

    pub fn set_static_bind_groups(&mut self, static_bind_groups: Vec<(u32, BindGroupId)>) {
        self.static_bind_groups = static_bind_groups;
        self.dirty = true;
    }

    fn empty(label: String) -> Self {
        Self {
            id: fastrand::usize(..),
            label,
            primitive_state: wgpu::PrimitiveState::default(),
            vertex_shader: None,
            fragment_shader: None,
            static_bind_groups: vec![],
            draw: RenderDraw::Direct {
                vertices: 0..3,
                instances: 0..1,
            },
            inner: None,
            dirty: true,
            has_error: false,
        }
    }

    fn new(
        label: String,
        ctx: &Context,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: Option<ShaderId>,
        fragment_shader: Option<ShaderId>,
        static_bind_groups: Vec<(u32, BindGroupId)>,
        draw: RenderDraw,
    ) -> AppResult<Self> {
        let inner = match Self::create_wgpu_pipeline(
            ctx,
            &label,
            &static_bind_groups,
            &draw,
            vertex_shader,
            fragment_shader,
            primitive_state,
            color_format,
            depth_format,
        ) {
            Ok(pipeline) => Some(pipeline),
            Err(AppError::UninitResource) => None,
            Err(e) => return Err(e),
        };

        Ok(Self {
            id: fastrand::usize(..),
            label,
            primitive_state,
            vertex_shader,
            fragment_shader,
            static_bind_groups,
            draw,
            inner,
            dirty: false,
            has_error: false,
        })
    }

    pub fn draw(&self, render_pass: &mut wgpu::RenderPass, ctx: &Context) -> AppResult<()> {
        let inner = self.inner.as_ref().ok_or(AppError::UninitResource)?;
        render_pass.set_pipeline(inner);

        for &(slot, id) in &self.static_bind_groups {
            let bind_group = ctx.bind_groups.get(id)?;
            render_pass.set_bind_group(slot, bind_group.inner(), &[]);
        }

        match &self.draw {
            RenderDraw::Model {
                model_id,
                instances,
                mesh_vertex_slot,
                material_bind_group_slot,
            } => {
                let model_id = model_id.ok_or(AppError::UninitResource)?;
                let model = ctx.models.get(model_id)?;

                for mesh in model.meshes() {
                    let vertex_buffer = mesh.vertex_buffer().inner();
                    render_pass.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer.slice(..));

                    if let Some(mat_slot) = material_bind_group_slot {
                        if let Some(material_index) = mesh.material_index() {
                            if let Some(material) = model.get_material(material_index) {
                                if let Some(bind_group_id) = material.bind_group_id() {
                                    let bind_group = ctx.bind_groups.get(bind_group_id)?;
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
        ctx: &Context,
        label: &str,
        static_bind_groups: &[(u32, BindGroupId)],
        draw: &RenderDraw,
        vertex_shader_id: Option<ShaderId>,
        fragment_shader_id: Option<ShaderId>,
        primitive_state: wgpu::PrimitiveState,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> AppResult<wgpu::RenderPipeline> {
        let vertex_shader_id = vertex_shader_id.ok_or(AppError::UninitResource)?;
        let fragment_shader_id = fragment_shader_id.ok_or(AppError::UninitResource)?;

        let bind_group_layouts = Self::resolved_bind_group_layout(ctx, &static_bind_groups, draw);
        let device = ctx.device;

        let scope = WgpuErrorScope::push(ctx.device);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });
        scope.pop()?;

        let resolved_attributes_and_stride = Self::resolved_attributes_and_stride(ctx, draw)?;
        let vertex_buffers: &[wgpu::VertexBufferLayout] = match &resolved_attributes_and_stride {
            Some((attributes, array_stride)) => &[wgpu::VertexBufferLayout {
                array_stride: *array_stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes,
            }],
            None => &[],
        };

        let vertex_shader = ctx.shaders.get(vertex_shader_id)?;
        let vertex_shader = vertex_shader.inner();

        let fragment_shader = ctx.shaders.get(fragment_shader_id)?;
        let fragment_shader = fragment_shader.inner();

        let scope = WgpuErrorScope::push(ctx.device);
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        });
        scope.pop()?;

        Ok(render_pipeline)
    }

    fn resolved_bind_group_layout<'a>(
        ctx: &'a Context,
        static_bind_groups: &[(u32, BindGroupId)],
        draw: &RenderDraw,
    ) -> Vec<Option<&'a wgpu::BindGroupLayout>> {
        let layouts: Vec<(u32, &'a wgpu::BindGroupLayout)> = static_bind_groups
            .iter()
            .copied()
            .filter_map(|(slot, bind_group_id)| {
                let bind_group = ctx.bind_groups.get(bind_group_id).ok()?;
                Some((slot, bind_group.inner_layout()))
            })
            .chain(draw.material_bind_group_slot_and_layout(ctx))
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
        ctx: &Context,
        draw: &RenderDraw,
    ) -> AppResult<Option<(Vec<wgpu::VertexAttribute>, u64)>> {
        match &draw {
            RenderDraw::Model { model_id, .. } => {
                let model_id = model_id.ok_or(AppError::UninitResource)?;
                let model = ctx.models.get(model_id)?;
                let spec = model.vertex_buffer_spec();
                Ok(Some(spec.to_wgpu_attributes_and_stride()))
            }
            RenderDraw::Direct { .. } => Ok(None),
        }
    }

    fn recreate(
        &mut self,
        ctx: &Context,
        tracker: &RecreateTracker,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        render_pass_dirty: bool,
    ) -> AppResult<()> {
        let shader_recreated = self
            .vertex_shader
            .is_some_and(|id| tracker.happened(ProjectEvent::ShaderRecreated(id)))
            || self
                .fragment_shader
                .is_some_and(|id| tracker.happened(ProjectEvent::ShaderRecreated(id)));

        let recreate = render_pass_dirty || self.dirty || self.has_error || shader_recreated;

        if !recreate {
            return Ok(());
        }

        match Self::create_wgpu_pipeline(
            ctx,
            &self.label,
            &self.static_bind_groups,
            &self.draw,
            self.vertex_shader,
            self.fragment_shader,
            self.primitive_state,
            color_format,
            depth_format,
        ) {
            Ok(pipeline) => {
                self.inner = Some(pipeline);
                self.has_error = false;
                self.dirty = false;
            }
            Err(AppError::UninitResource) => {
                self.inner = None;
                self.has_error = false;
                self.dirty = false;
            }
            Err(e) => {
                self.has_error = true;
                return Err(e);
            }
        }

        Ok(())
    }
}

impl<T> RenderPassTarget<T> {
    pub fn get_target_inner<'a>(
        &self,
        texture_views: &'a Storage<TextureViewId, TextureView>,
    ) -> AppResult<&'a wgpu::TextureView> {
        let target_view_id = self.texture_view_id.ok_or(AppError::UninitResource)?;
        let target_view = texture_views.get(target_view_id)?;
        let inner = target_view.inner().as_ref();
        let inner = inner.ok_or(AppError::UninitResourceOther(target_view_id.into()))?;

        Ok(inner)
    }
}

impl RenderDraw {
    pub fn material_bind_group_slot_and_layout<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> Option<(u32, &'a wgpu::BindGroupLayout)> {
        let RenderDraw::Model {
            model_id: Some(model_id),
            material_bind_group_slot: Some(slot),
            ..
        } = self
        else {
            return None;
        };
        let model = ctx.models.get(*model_id).ok()?;
        let layout = model.get_bind_group_layout(ctx.bind_groups)?;
        Some((*slot, layout))
    }
}

impl Recreatable for RenderPass {
    type Context<'a> = Context<'a>;
    type Id = RenderPassId;

    fn recreate<'a>(
        &mut self,
        _id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        let color_format = self.get_color_format(ctx)?;
        let depth_format = self.get_depth_format(ctx)?;

        for ele in &mut self.pipelines {
            ele.recreate(ctx, tracker, color_format, depth_format, self.dirty)?;
        }

        self.dirty = false;

        Ok(None)
    }
}

impl Default for LoadOperation<Color> {
    fn default() -> Self {
        LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0]))
    }
}

impl Default for LoadOperation<f32> {
    fn default() -> Self {
        LoadOperation::Clear(1.0)
    }
}

impl<T> Default for RenderPassTarget<T>
where
    LoadOperation<T>: Default,
{
    fn default() -> Self {
        RenderPassTarget {
            texture_view_id: None,
            load_operation: LoadOperation::default(),
        }
    }
}

impl<T, V> From<LoadOperation<T>> for wgpu::LoadOp<V>
where
    T: Into<V>,
{
    fn from(value: LoadOperation<T>) -> Self {
        match value {
            LoadOperation::Clear(value) => wgpu::LoadOp::Clear(value.into()),
            LoadOperation::Load => wgpu::LoadOp::Load,
        }
    }
}

impl From<Color> for wgpu::Color {
    fn from(Color(value): Color) -> Self {
        wgpu::Color {
            r: value[0] as f64,
            g: value[1] as f64,
            b: value[2] as f64,
            a: value[3] as f64,
        }
    }
}

impl std::hash::Hash for RenderPipeline {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
