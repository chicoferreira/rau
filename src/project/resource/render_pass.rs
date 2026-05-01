use egui_dnd::utils::shift_vec;

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, Creatable, ModelId, ProjectResource, RenderPassId, ShaderId, TextureViewId,
        resource::{bindgroup::BindGroup, model::Model, shader::Shader, texture_view::TextureView},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
};

pub struct RenderPass {
    pub label: String,
    pub target: RenderPassTarget<Color>,
    pub depth_target: Option<RenderPassTarget<f32>>,
    pub pipelines: Vec<RenderPipeline>,
    revision: Revision,
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

pub struct RenderPassRuntime {
    pub runtime_pipelines: Vec<RenderPipelineRuntime>,
}

pub struct RenderPipelineRuntime {
    inner: wgpu::RenderPipeline,
}

pub const MAX_RENDER_PASS_BIND_GROUPS: usize = 8;

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
    pub static_bind_groups: Vec<(u32, Option<BindGroupId>)>,
    pub draw: RenderDraw,
    dirty: bool,
}

#[derive(Clone, Debug, PartialEq)]
// TODO: rename this to RenderDrawStrategy
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

impl Default for RenderDraw {
    fn default() -> Self {
        Self::Direct {
            vertices: 0..3,
            instances: 0..1,
        }
    }
}

impl Creatable for RenderPass {
    const DEFAULT_LABEL: &'static str = "Render Pass";

    fn create(label: String) -> Self {
        Self {
            label,
            target: Default::default(),
            depth_target: Default::default(),
            pipelines: Default::default(),
            revision: Default::default(),
        }
    }
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub models: &'a Storage<Model>,
    pub runtime_models: &'a RuntimeStorage<Model>,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
}

impl ProjectResource for RenderPass {
    type Id = RenderPassId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl RenderPass {
    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }

    pub fn set_target(&mut self, target: RenderPassTarget<Color>) {
        self.target = target;
        self.revision.increase();
    }

    pub fn set_depth_target(&mut self, target: Option<RenderPassTarget<f32>>) {
        self.depth_target = target;
        self.revision.increase();
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
            revision: Revision::default(),
        }
    }

    pub fn add_pipeline(
        &mut self,
        label: impl Into<String>,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: Option<ShaderId>,
        fragment_shader: Option<ShaderId>,
        static_bind_groups: Vec<(u32, Option<BindGroupId>)>,
        draw: RenderDraw,
    ) {
        let pipeline = RenderPipeline::new(
            label.into(),
            primitive_state,
            vertex_shader,
            fragment_shader,
            static_bind_groups,
            draw,
        );
        self.pipelines.push(pipeline);
        self.revision.increase();
    }

    pub fn add_empty_pipeline(&mut self, label: impl Into<String>) {
        self.add_pipeline(
            label,
            wgpu::PrimitiveState::default(),
            None,
            None,
            vec![],
            RenderDraw::default(),
        );
    }

    pub fn remove_pipeline(&mut self, index: usize) {
        if index < self.pipelines.len() {
            self.pipelines.remove(index);
            self.revision.increase();
        }
    }

    pub fn reorder_pipelines(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.pipelines);
        self.revision.increase();
    }

    pub fn get_color_format(&self, ctx: &Context) -> AppResult<wgpu::TextureFormat> {
        let target_view = self.target.get_target_inner(ctx.runtime_texture_views)?;
        Ok(target_view.texture().format())
    }

    pub fn get_depth_format(&self, ctx: &Context) -> AppResult<Option<wgpu::TextureFormat>> {
        Ok(self
            .depth_target
            .as_ref()
            .map(|target| target.get_target_inner(ctx.runtime_texture_views))
            .transpose()?
            .map(|view| view.texture().format()))
    }

    pub fn submit(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ctx: &Context,
        runtime: &RenderPassRuntime,
    ) -> AppResult<()> {
        let color_view = self.target.get_target_inner(ctx.runtime_texture_views)?;

        let depth_stencil_attachment = self
            .depth_target
            .as_ref()
            .map(|depth_target| -> AppResult<_> {
                let depth_view = depth_target.get_target_inner(ctx.runtime_texture_views)?;
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

        for (pipeline, pipeline_runtime) in self.pipelines.iter().zip(&runtime.runtime_pipelines) {
            pipeline_runtime.draw(&mut render_pass, pipeline, ctx)?;
        }

        Ok(())
    }
}

impl RenderPipeline {
    fn new(
        label: String,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: Option<ShaderId>,
        fragment_shader: Option<ShaderId>,
        static_bind_groups: Vec<(u32, Option<BindGroupId>)>,
        draw: RenderDraw,
    ) -> Self {
        Self {
            id: fastrand::usize(..),
            label,
            primitive_state,
            vertex_shader,
            fragment_shader,
            static_bind_groups,
            draw,
            dirty: false,
        }
    }

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

    pub fn set_static_bind_groups(&mut self, static_bind_groups: Vec<(u32, Option<BindGroupId>)>) {
        self.static_bind_groups = static_bind_groups;
        self.dirty = true;
    }

    fn create_wgpu_pipeline(
        ctx: &Context,
        label: &str,
        static_bind_groups: &[(u32, Option<BindGroupId>)],
        draw: &RenderDraw,
        vertex_shader_id: Option<ShaderId>,
        fragment_shader_id: Option<ShaderId>,
        primitive_state: wgpu::PrimitiveState,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> AppResult<wgpu::RenderPipeline> {
        let vertex_shader_id = vertex_shader_id.ok_or(AppError::UninitializedFields)?;
        let fragment_shader_id = fragment_shader_id.ok_or(AppError::UninitializedFields)?;

        let bind_group_layouts = Self::resolved_bind_group_layout(ctx, &static_bind_groups, draw)?;
        let device = ctx.device;

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &bind_group_layouts,
            immediate_size: 0,
        });

        let resolved_attributes_and_stride = Self::resolved_attributes_and_stride(ctx, draw)?;
        let vertex_buffers: &[wgpu::VertexBufferLayout] = match &resolved_attributes_and_stride {
            Some((attributes, array_stride)) => &[wgpu::VertexBufferLayout {
                array_stride: *array_stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes,
            }],
            None => &[],
        };

        let vertex_shader = ctx.runtime_shaders.get_init(vertex_shader_id)?;
        let vertex_shader = vertex_shader.inner();

        let fragment_shader = ctx.runtime_shaders.get_init(fragment_shader_id)?;
        let fragment_shader = fragment_shader.inner();

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

        Ok(render_pipeline)
    }

    fn resolved_bind_group_layout<'a>(
        ctx: &'a Context,
        static_bind_groups: &[(u32, Option<BindGroupId>)],
        draw: &RenderDraw,
    ) -> AppResult<Vec<Option<&'a wgpu::BindGroupLayout>>> {
        let material_layout = draw.material_bind_group_slot_and_layout(ctx)?;
        let max_slot = static_bind_groups
            .iter()
            .map(|(slot, _)| *slot)
            .chain(material_layout.iter().map(|(slot, _)| *slot))
            .max();

        let Some(max_slot) = max_slot else {
            return Ok(vec![]);
        };

        let layout_count = max_slot as usize + 1;
        if layout_count > MAX_RENDER_PASS_BIND_GROUPS {
            return Err(AppError::BindGroupLayoutLimitExceeded {
                count: layout_count,
                max: MAX_RENDER_PASS_BIND_GROUPS,
            });
        }

        let mut result = vec![None; max_slot as usize + 1];

        for (slot, bind_group_id) in static_bind_groups.iter().copied() {
            let Some(bind_group_id) = bind_group_id else {
                continue;
            };
            let bind_group = ctx.runtime_bind_groups.get_init(bind_group_id)?;
            result[slot as usize] = Some(bind_group.inner_layout());
        }

        if let Some((slot, layout)) = material_layout {
            result[slot as usize] = Some(layout);
        }

        Ok(result)
    }

    fn resolved_attributes_and_stride(
        ctx: &Context,
        draw: &RenderDraw,
    ) -> AppResult<Option<(Vec<wgpu::VertexAttribute>, u64)>> {
        match &draw {
            RenderDraw::Model { model_id, .. } => {
                let model_id = model_id.ok_or(AppError::UninitializedFields)?;
                let model = ctx.models.get(model_id)?;
                let spec = model.vertex_buffer_spec();
                Ok(Some(spec.to_wgpu_attributes_and_stride()))
            }
            RenderDraw::Direct { .. } => Ok(None),
        }
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        [self.vertex_shader, self.fragment_shader]
            .into_iter()
            .flatten()
            .any(|shader| tracker.was_changed(shader))
            || self
                .static_bind_groups
                .iter()
                .flat_map(|(_, bind_group_id)| *bind_group_id)
                .any(|bind_group_id| tracker.was_changed(bind_group_id))
            || self.draw.needs_rebuild_from_others(tracker)
    }
}

impl RenderPipelineRuntime {
    pub fn draw(
        &self,
        render_pass: &mut wgpu::RenderPass,
        pipeline: &RenderPipeline,
        ctx: &Context,
    ) -> AppResult<()> {
        render_pass.set_pipeline(&self.inner);

        for &(slot, id) in &pipeline.static_bind_groups {
            let Some(id) = id else {
                continue;
            };
            let bind_group = ctx.runtime_bind_groups.get_init(id)?;
            render_pass.set_bind_group(slot, bind_group.inner(), &[]);
        }

        match &pipeline.draw {
            RenderDraw::Model {
                model_id,
                instances,
                mesh_vertex_slot,
                material_bind_group_slot,
            } => {
                let model_id = model_id.ok_or(AppError::UninitializedFields)?;
                let model = ctx.models.get(model_id)?;
                let model_runtime = ctx.runtime_models.get_init(model_id)?;

                for (mesh_index, mesh) in model_runtime.meshes().iter().enumerate() {
                    let vertex_buffer = mesh.vertex_buffer().inner();
                    render_pass.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer.slice(..));

                    if let Some(mat_slot) = material_bind_group_slot {
                        let material_index = model
                            .selected_material_index(mesh_index, mesh)
                            .ok_or(AppError::UninitializedFields)?;
                        model_runtime
                            .get_material(material_index)
                            .ok_or(AppError::UninitializedFields)?;
                        let bind_group_id = model
                            .material_bind_group_id(material_index)
                            .ok_or(AppError::UninitializedFields)?;
                        let bind_group = ctx.runtime_bind_groups.get_init(bind_group_id)?;
                        render_pass.set_bind_group(*mat_slot, bind_group.inner(), &[]);
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
}

impl<T> RenderPassTarget<T> {
    pub fn get_target_inner<'a>(
        &self,
        runtime_texture_views: &'a RuntimeStorage<TextureView>,
    ) -> AppResult<&'a wgpu::TextureView> {
        let target_view_id = self.texture_view_id.ok_or(AppError::UninitializedFields)?;
        let target_view = runtime_texture_views.get_init(target_view_id)?;
        Ok(target_view.inner())
    }
}

impl RenderDraw {
    pub fn material_bind_group_slot_and_layout<'a>(
        &self,
        ctx: &Context<'a>,
    ) -> AppResult<Option<(u32, &'a wgpu::BindGroupLayout)>> {
        let RenderDraw::Model {
            model_id,
            material_bind_group_slot,
            ..
        } = self
        else {
            return Ok(None);
        };

        let Some(slot) = material_bind_group_slot else {
            return Ok(None);
        };

        let model_id = (*model_id).ok_or(AppError::UninitializedFields)?;
        let model = ctx.models.get(model_id)?;
        let model_runtime = ctx.runtime_models.get_init(model_id)?;
        let layout = model_runtime.get_bind_group_layout(model, ctx.runtime_bind_groups)?;
        Ok(Some((*slot, layout)))
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        match self {
            RenderDraw::Model { model_id, .. } => {
                model_id.is_some_and(|id| tracker.was_changed(id))
            }
            RenderDraw::Direct { .. } => false,
        }
    }
}

impl SyncResource for RenderPass {
    type Context<'a> = Context<'a>;
    type Runtime = RenderPassRuntime;

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let color_format = self.get_color_format(ctx)?;
        let depth_format = self.get_depth_format(ctx)?;

        let runtime_pipelines: AppResult<Vec<RenderPipelineRuntime>> = self
            .pipelines
            .iter()
            .map(|pipeline| {
                RenderPipeline::create_wgpu_pipeline(
                    ctx,
                    &pipeline.label,
                    &pipeline.static_bind_groups,
                    &pipeline.draw,
                    pipeline.vertex_shader,
                    pipeline.fragment_shader,
                    pipeline.primitive_state,
                    color_format,
                    depth_format,
                )
            })
            .map(|wgpu_pipeline| wgpu_pipeline.map(|inner| RenderPipelineRuntime { inner }))
            .collect();

        let runtime_pipelines = runtime_pipelines?;

        Ok(SyncOutcome::Changed(RenderPassRuntime {
            runtime_pipelines,
        }))
    }

    fn after_sync(&mut self) {
        for pipeline in &mut self.pipelines {
            pipeline.dirty = false;
        }
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.target
            .texture_view_id
            .is_some_and(|id| tracker.was_changed(id))
            || self
                .depth_target
                .as_ref()
                .and_then(|target| target.texture_view_id)
                .is_some_and(|id| tracker.was_changed(id))
            || self
                .pipelines
                .iter()
                .any(|pipeline| pipeline.dirty || pipeline.needs_rebuild_from_others(tracker))
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
