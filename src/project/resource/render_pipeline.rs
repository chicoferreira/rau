use std::{ops::Range, task::Poll};

use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult, RequiredFieldExt},
    project::{
        BindGroupId, Creatable, ModelId, ProjectResource, RenderPipelineId, ShaderId,
        resource::{bindgroup::BindGroup, model::Model, shader::Shader},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::{
        async_job::AsyncJob, validate_bind_group_layouts::validate_bind_group_layouts,
        vec_set_at_extension::VecSetAtExtension, wgpu_error_scope::WgpuErrorScope,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPipeline {
    label: String,
    primitive_state: wgpu::PrimitiveState,
    vertex_shader: Option<ShaderId>,
    fragment_shader: Option<ShaderId>,
    draw_strategy: RenderDrawStrategy,
    static_bind_groups: Vec<(u32, Option<BindGroupId>)>,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RenderDrawStrategy {
    /// Loop through all the model's meshes, setting the mesh
    /// material bind group at slot `material_bind_group_slot`
    /// and drawing the vertices/instances from the mesh
    Model {
        model_id: Option<ModelId>,
        instances: Range<u32>,
        mesh_vertex_slot: u32,
        material_bind_group_slot: Option<u32>,
    },
    /// Draw a number of vertices and instances directly without underlying buffers
    Direct {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
}

impl RenderPipeline {
    pub fn new(
        label: impl Into<String>,
        primitive_state: wgpu::PrimitiveState,
        vertex_shader: Option<ShaderId>,
        fragment_shader: Option<ShaderId>,
        draw_strategy: RenderDrawStrategy,
        static_bind_groups: Vec<(u32, Option<BindGroupId>)>,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
    ) -> Self {
        Self {
            label: label.into(),
            primitive_state,
            vertex_shader,
            fragment_shader,
            draw_strategy,
            static_bind_groups,
            color_format,
            depth_format,
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }

    resource_getters! {
        pub fn static_bind_groups() -> &[(u32, Option<BindGroupId>)];
        pub fn primitive_state() -> wgpu::PrimitiveState;
        pub fn vertex_shader() -> Option<ShaderId>;
        pub fn fragment_shader() -> Option<ShaderId>;
        pub fn draw_strategy() -> &RenderDrawStrategy;
        pub fn color_format() -> wgpu::TextureFormat;
        pub fn depth_format() -> Option<wgpu::TextureFormat>;
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_label(label: String);
        pub fn set_primitive_state(primitive_state: wgpu::PrimitiveState);
        pub fn set_vertex_shader(vertex_shader: Option<ShaderId>);
        pub fn set_fragment_shader(fragment_shader: Option<ShaderId>);
        pub fn set_draw_strategy(draw_strategy: RenderDrawStrategy);
        pub fn set_static_bind_groups(static_bind_groups: Vec<(u32, Option<BindGroupId>)>);
        pub fn set_color_format(color_format: wgpu::TextureFormat);
        pub fn set_depth_format(depth_format: Option<wgpu::TextureFormat>);
    }

    pub fn collect_bind_group_ids(
        &self,
        models: &Storage<Model>,
    ) -> AppResult<Vec<Option<BindGroupId>>> {
        let mut bind_group_ids = vec![];

        for (slot, id) in &self.static_bind_groups {
            let bind_group_id = id.ok_or_uninit_field(format!("Bind Group Id at Slot {slot}"))?;

            bind_group_ids.set_at(*slot as usize, Some(bind_group_id));
        }

        if let RenderDrawStrategy::Model {
            model_id,
            material_bind_group_slot,
            ..
        } = &self.draw_strategy
        {
            let model_id = model_id.ok_or_uninit_field("Render Draw Strategy Model Id")?;

            let model = models.get(model_id)?;

            if let Some(slot) = material_bind_group_slot {
                // Model already checks that all material bind group ids have the same layout when syncing
                let first_material_bg_id = model.get_material_bind_group_ids().first().cloned();
                if let Some(first_material_bind_group_id) = first_material_bg_id {
                    bind_group_ids.set_at(*slot as usize, Some(first_material_bind_group_id));
                }
            }
        }

        Ok(bind_group_ids)
    }
}

pub struct RenderPipelineRuntime {
    pub inner: wgpu::RenderPipeline,
}

impl Creatable for RenderPipeline {
    fn create(label: String) -> Self {
        Self {
            label,
            primitive_state: Default::default(),
            vertex_shader: None,
            fragment_shader: None,
            draw_strategy: RenderDrawStrategy::Direct {
                vertices: 0..3,
                instances: 0..1,
            },
            static_bind_groups: Vec::new(),
            color_format: wgpu::TextureFormat::Rgba8UnormSrgb,
            depth_format: None,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }
}

impl ProjectResource for RenderPipeline {
    type Id = RenderPipelineId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

#[derive(Default)]
pub enum RenderPipelineCreationJob {
    #[default]
    Start,
    Validation(AsyncJob<AppResult<()>>, RenderPipelineRuntime),
}

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub runtime_shaders: &'a RuntimeStorage<Shader>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub models: &'a Storage<Model>,
    pub runtime_models: &'a RuntimeStorage<Model>,
}

impl SyncResource for RenderPipeline {
    type Context<'a> = Context<'a>;

    type Runtime = RenderPipelineRuntime;

    type Job = RenderPipelineCreationJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        let draw_strategy_needs_rebuild = match self.draw_strategy {
            RenderDrawStrategy::Model { model_id, .. } => {
                model_id.is_some_and(|id| tracker.was_changed(id))
            }
            RenderDrawStrategy::Direct { .. } => false,
        };

        let shaders_needs_rebuild = [self.vertex_shader, self.fragment_shader]
            .into_iter()
            .any(|id| id.is_some_and(|id| tracker.was_changed(id)));

        let static_bind_groups_needs_rebuild = self
            .static_bind_groups
            .iter()
            .any(|(_, id)| id.is_some_and(|id| tracker.was_changed(id)));

        draw_strategy_needs_rebuild || shaders_needs_rebuild || static_bind_groups_needs_rebuild
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        if let RenderPipelineCreationJob::Validation(mut future, runtime) = job {
            return match future.try_resolve() {
                Poll::Pending => {
                    let job = RenderPipelineCreationJob::Validation(future, runtime);
                    Ok(SyncOutcome::Pending(job))
                }
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
            };
        }

        let vertex_shader_id = self.vertex_shader.ok_or_uninit_field("Vertex Shader")?;
        let fragment_shader_id = self.fragment_shader.ok_or_uninit_field("Fragment Shader")?;

        validate_polygon_mode(ctx.device.features(), self.primitive_state.polygon_mode)?;

        let vertex_shader = ctx.runtime_shaders.get_init(vertex_shader_id)?;
        let fragment_shader = ctx.runtime_shaders.get_init(fragment_shader_id)?;

        let (Some(vertex_shader), Some(fragment_shader)) = (vertex_shader, fragment_shader) else {
            // Shaders aren't ready yet, restart the sync job on the next frame
            return Ok(SyncOutcome::Pending(RenderPipelineCreationJob::Start));
        };

        let vertex_attributes_and_stride = match &self.draw_strategy {
            RenderDrawStrategy::Model { model_id, .. } => {
                let model_id = model_id.ok_or_uninit_field("Draw Strategy Model Id")?;
                let model = ctx.models.get(model_id)?;
                let spec = model.vertex_buffer_spec();
                Some(spec.to_wgpu_attributes_and_stride())
            }
            RenderDrawStrategy::Direct { .. } => None,
        };

        let vertex_buffers: &[wgpu::VertexBufferLayout] = match &vertex_attributes_and_stride {
            Some((attributes, array_stride)) => &[wgpu::VertexBufferLayout {
                array_stride: *array_stride,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes,
            }],
            None => &[],
        };

        let bind_group_ids = self.collect_bind_group_ids(ctx.models)?;

        let mut bind_group_layouts = vec![];
        for id in &bind_group_ids {
            let result = match id {
                Some(id) => {
                    let Some(bind_group) = ctx.runtime_bind_groups.get_init(*id)? else {
                        // Bind groups aren't ready yet, restart the sync job on the next frame
                        return Ok(SyncOutcome::Pending(RenderPipelineCreationJob::Start));
                    };

                    Some(bind_group.inner_layout())
                }
                None => None,
            };

            bind_group_layouts.push(result);
        }

        validate_bind_group_layouts(&bind_group_layouts, &ctx.device.limits())?;

        let scope = WgpuErrorScope::push(ctx.device);

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(&format!("{} Pipeline Layout", self.label)),
                bind_group_layouts: &bind_group_layouts,
                immediate_size: 0,
            });

        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: Some(&self.label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: vertex_shader.inner(),
                entry_point: None, // TODO: maybe allow for users to specify the entrypoint later?
                buffers: vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: fragment_shader.inner(),
                entry_point: None, // TODO: maybe allow for users to specify the entrypoint later?
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.color_format,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: self.primitive_state,
            depth_stencil: self.depth_format.map(|format| wgpu::DepthStencilState {
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
        };

        let render_pipeline = ctx
            .device
            .create_render_pipeline(&render_pipeline_descriptor);

        let runtime = RenderPipelineRuntime {
            inner: render_pipeline,
        };

        let job = RenderPipelineCreationJob::Validation(scope.pop(), runtime);
        Ok(SyncOutcome::Pending(job))
    }
}

fn validate_polygon_mode(
    features: wgpu::Features,
    polygon_mode: wgpu::PolygonMode,
) -> AppResult<()> {
    match polygon_mode {
        wgpu::PolygonMode::Fill => {}
        wgpu::PolygonMode::Line => {
            if !features.contains(wgpu::Features::POLYGON_MODE_LINE) {
                return Err(AppError::UnsupportedRendererFeature("Line Polygon Mode"));
            }
        }
        wgpu::PolygonMode::Point => {
            if !features.contains(wgpu::Features::POLYGON_MODE_POINT) {
                return Err(AppError::UnsupportedRendererFeature("Point Polygon Mode"));
            }
        }
    }

    Ok(())
}
