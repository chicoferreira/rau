use std::{ops::Range, task::Poll};

use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult, RequiredFieldExt},
    project::{
        Creatable, ProjectResource, RenderPassId, RenderPipelineId, TextureViewId,
        resource::{
            bindgroup::BindGroup,
            model::{Mesh, Model},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            texture_view::{TextureView, TextureViewRuntime},
        },
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

pub struct Context<'a> {
    pub device: &'a wgpu::Device,
    pub models: &'a Storage<Model>,
    pub render_pipelines: &'a Storage<RenderPipeline>,
    pub runtime_models: &'a RuntimeStorage<Model>,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub runtime_texture_views: &'a RuntimeStorage<TextureView>,
    pub runtime_render_pipelines: &'a RuntimeStorage<RenderPipeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPass {
    label: String,
    targets: Vec<RenderPassTarget<Color>>,
    depth_target: Option<RenderPassTarget<f32>>,
    pipelines: Vec<RenderPipelineId>,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderPassTarget<T> {
    texture_view_id: Option<TextureViewId>,
    load_operation: LoadOperation<T>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum LoadOperation<T> {
    Clear(T),
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", transparent)]
pub struct Color(pub [f32; 4]);

pub struct RenderPassRuntime {
    bundle: wgpu::RenderBundle,
}

#[derive(Default)]
pub enum RenderPassJob {
    #[default]
    Start,
    Validation(RenderPassRuntime, AsyncJob<AppResult<()>>),
}

impl Creatable for RenderPass {
    fn create(label: String) -> Self {
        Self {
            label,
            targets: vec![RenderPassTarget::default()],
            depth_target: Default::default(),
            pipelines: Default::default(),
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }
}

impl ProjectResource for RenderPass {
    type Id = RenderPassId;

    fn label(&self) -> &str {
        &self.label
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        fn set_label(label: String);
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl RenderPass {
    pub fn new(
        label: impl Into<String>,
        targets: Vec<RenderPassTarget<Color>>,
        depth_target: Option<RenderPassTarget<f32>>,
    ) -> Self {
        Self {
            label: label.into(),
            targets,
            depth_target,
            pipelines: Default::default(),
            runtime_revision: Default::default(),
            project_revision: Default::default(),
        }
    }

    resource_getters! {
        pub fn targets() -> &[RenderPassTarget<Color>];
        pub fn depth_target() -> Option<RenderPassTarget<f32>>;
        pub fn pipelines() -> &[RenderPipelineId];
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_targets(targets: Vec<RenderPassTarget<Color>>);
        pub fn set_depth_target(depth_target: Option<RenderPassTarget<f32>>);
        pub fn set_pipelines(pipelines: Vec<RenderPipelineId>);
    }

    /// The texture views this pass renders into, color first.
    fn target_texture_view_ids(&self) -> impl Iterator<Item = TextureViewId> {
        let depth = self.depth_target.as_ref();
        self.targets
            .iter()
            .filter_map(|target| target.texture_view_id)
            .into_iter()
            .chain(depth.and_then(|target| target.texture_view_id))
    }

    pub fn map_color_targets<'a, R>(
        &self,
        runtime_texture_views: &'a RuntimeStorage<TextureView>,
        f: impl Fn(&RenderPassTarget<Color>, &'a TextureViewRuntime) -> R,
    ) -> AppResult<Option<Vec<Option<R>>>> {
        self.targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let field = || format!("Color Target Texture #{index}");
                let view = target.resolve_view(runtime_texture_views, field)?;

                Ok(view.map(|view| Some(f(target, view))))
            })
            .collect()
    }

    pub fn map_depth_target<'a, R>(
        &self,
        runtime_texture_views: &'a RuntimeStorage<TextureView>,
        f: impl FnOnce(&RenderPassTarget<f32>, &'a TextureViewRuntime) -> R,
    ) -> AppResult<Option<Option<R>>> {
        let Some(target) = &self.depth_target else {
            return Ok(Some(None));
        };

        let field = || "Depth Target Texture".to_string();
        let view = target.resolve_view(runtime_texture_views, field)?;

        Ok(view.map(|view| Some(f(target, view))))
    }

    /// Collects every runtime resource the pipelines record with.
    ///
    /// Returns `Ok(None)` if a runtime resource (pipeline, bind group, model) is
    /// still rebuilding.
    fn collect_draws<'a>(&self, ctx: &Context<'a>) -> AppResult<Option<Vec<PipelineDraw<'a>>>> {
        let Context {
            models,
            render_pipelines,
            runtime_models,
            runtime_bind_groups,
            runtime_render_pipelines,
            ..
        } = *ctx;

        let mut draws = vec![];
        for id in &self.pipelines {
            let pipeline = render_pipelines.get(*id)?;
            let Some(pipeline_runtime) = runtime_render_pipelines.get_init(*id)? else {
                return Ok(None); // pending: pipeline still rebuilding
            };

            let mut bind_groups = vec![];
            let mut material_bind_group_slots = vec![];
            for (slot, bind_group_target) in pipeline.bind_groups().iter().enumerate() {
                let slot = slot as u32;
                match bind_group_target {
                    BindGroupTarget::Empty => bind_groups.push((slot, None)),
                    BindGroupTarget::Static(id) => {
                        let Some(bind_group) = runtime_bind_groups.get_init(*id)? else {
                            return Ok(None); // pending: static bind group not ready
                        };
                        bind_groups.push((slot, Some(bind_group.inner())));
                    }
                    BindGroupTarget::ModelMaterial => {
                        material_bind_group_slots.push(slot);
                    }
                }
            }

            let kind = match pipeline.draw_strategy() {
                RenderDrawStrategy::Model {
                    model_id,
                    instances,
                    mesh_vertex_slot,
                } => {
                    let model_id = model_id
                        .ok_or_uninit_field(format!("Pipeline {} Model Id", pipeline.label()))?;

                    let model = models.get(model_id)?;
                    let Some(model_runtime) = runtime_models.get_init(model_id)? else {
                        return Ok(None); // pending: model not ready
                    };

                    let mut meshes = vec![];
                    for (mesh_index, mesh) in model_runtime.meshes().iter().enumerate() {
                        if material_bind_group_slots.is_empty() {
                            meshes.push((mesh, None));
                            continue;
                        }

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

                        let Some(bind_group) = runtime_bind_groups.get_init(bind_group_id)? else {
                            return Ok(None); // pending: material bind group not ready
                        };

                        meshes.push((mesh, Some(bind_group.inner())));
                    }

                    DrawKind::Model {
                        meshes,
                        material_slots: material_bind_group_slots,
                        mesh_vertex_slot: *mesh_vertex_slot,
                        instances: instances.clone(),
                    }
                }
                RenderDrawStrategy::Direct {
                    vertices,
                    instances,
                } => DrawKind::Direct {
                    vertices: vertices.clone(),
                    instances: instances.clone(),
                },
            };

            draws.push(PipelineDraw {
                inner: &pipeline_runtime.inner,
                bind_groups,
                kind,
            });
        }

        Ok(Some(draws))
    }

    /// Records every pipeline's draw commands into `encoder`.
    fn record<'enc>(draws: &[PipelineDraw<'enc>], encoder: &mut wgpu::RenderBundleEncoder<'enc>) {
        for draw in draws {
            encoder.set_pipeline(draw.inner);

            for (slot, bind_group) in &draw.bind_groups {
                encoder.set_bind_group(*slot, *bind_group, &[]);
            }

            match &draw.kind {
                DrawKind::Model {
                    meshes,
                    material_slots,
                    mesh_vertex_slot,
                    instances,
                } => {
                    for (mesh, material_bind_group) in meshes {
                        let vertex_buffer = mesh.vertex_buffer().inner().slice(..);
                        encoder.set_vertex_buffer(*mesh_vertex_slot, vertex_buffer);

                        let index_buffer = mesh.index_buffer().inner().slice(..);
                        encoder.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint32);

                        if let Some(bind_group) = material_bind_group {
                            for slot in material_slots {
                                encoder.set_bind_group(*slot, *bind_group, &[]);
                            }
                        }

                        let index_num = mesh.indices().len() as u32;
                        encoder.draw_indexed(0..index_num, 0, instances.clone());
                    }
                }
                DrawKind::Direct {
                    vertices,
                    instances,
                } => encoder.draw(vertices.clone(), instances.clone()),
            }
        }
    }
}

struct PipelineDraw<'a> {
    inner: &'a wgpu::RenderPipeline,
    bind_groups: Vec<(u32, Option<&'a wgpu::BindGroup>)>,
    kind: DrawKind<'a>,
}

enum DrawKind<'a> {
    Model {
        meshes: Vec<(&'a Mesh, Option<&'a wgpu::BindGroup>)>,
        material_slots: Vec<u32>,
        mesh_vertex_slot: u32,
        instances: Range<u32>,
    },
    Direct {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
}

impl RenderPassRuntime {
    pub fn bundle(&self) -> &wgpu::RenderBundle {
        &self.bundle
    }
}

impl SyncResource for RenderPass {
    type Context<'a> = Context<'a>;
    type Runtime = RenderPassRuntime;
    type Job = RenderPassJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn needs_rebuild(&self, _: Self::Id, _: &Self::Context<'_>, tracker: &SyncTracker) -> bool {
        let targets_recreated = self
            .target_texture_view_ids()
            .any(|id| tracker.was_recreated(id));

        let pipelines_recreated = self
            .pipelines
            .iter()
            .any(|pipeline_id| tracker.was_recreated(*pipeline_id));

        targets_recreated || pipelines_recreated
    }

    fn sync<'a>(
        &self,
        _id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        if let RenderPassJob::Validation(runtime, mut future) = job {
            return match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Recreated(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(RenderPassJob::Validation(
                    runtime, future,
                ))),
            };
        }

        let Some(color_formats) =
            self.map_color_targets(ctx.runtime_texture_views, |_, view| view.format())?
        else {
            return Ok(SyncOutcome::Pending(RenderPassJob::Start));
        };

        let Some(depth_stencil) = self.map_depth_target(ctx.runtime_texture_views, |_, view| {
            wgpu::RenderBundleDepthStencil {
                format: view.format(),
                depth_read_only: false,
                stencil_read_only: true,
            }
        })?
        else {
            return Ok(SyncOutcome::Pending(RenderPassJob::Start));
        };

        let Some(draws) = self.collect_draws(ctx)? else {
            return Ok(SyncOutcome::Pending(RenderPassJob::Start));
        };

        let scope = WgpuErrorScope::push(ctx.device);

        let mut encoder =
            ctx.device
                .create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                    label: Some(&self.label),
                    color_formats: &color_formats,
                    depth_stencil,
                    sample_count: 1,
                    multiview: None,
                });

        Self::record(&draws, &mut encoder);

        let bundle = encoder.finish(&wgpu::RenderBundleDescriptor {
            label: Some(&self.label),
        });

        let runtime = RenderPassRuntime { bundle };
        let job = RenderPassJob::Validation(runtime, scope.pop());
        self.sync(_id, ctx, None, job)
    }
}

impl<T> RenderPassTarget<T> {
    /// Resolves the texture view this target renders into, naming it `field` in
    /// the error when it is unset.
    ///
    /// Returns `Ok(None)` if the view is still rebuilding.
    fn resolve_view<'a>(
        &self,
        runtime_texture_views: &'a RuntimeStorage<TextureView>,
        field: impl FnOnce() -> String,
    ) -> AppResult<Option<&'a TextureViewRuntime>> {
        let id = self
            .texture_view_id
            .ok_or_else(|| AppError::uninit_field(field()))?;

        runtime_texture_views.get_init(id)
    }

    pub fn new(texture_view_id: Option<TextureViewId>, load_operation: LoadOperation<T>) -> Self {
        Self {
            texture_view_id,
            load_operation,
        }
    }

    pub fn texture_view_id(&self) -> Option<TextureViewId> {
        self.texture_view_id
    }

    pub fn load_operation(&self) -> LoadOperation<T>
    where
        T: Copy,
    {
        self.load_operation
    }
}

impl<T> std::hash::Hash for RenderPassTarget<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.texture_view_id.hash(state);
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
