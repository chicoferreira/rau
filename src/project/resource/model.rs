use std::task::Poll;

use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

use crate::{
    error::{AppError, AppResult},
    file::file_storage::FileStorage,
    project::{
        BindGroupId, Creatable, ModelId, ProjectResource,
        paths::FilePath,
        resource::{bindgroup::BindGroup, model::vertex_buffer::VertexBufferSpec},
        storage::RuntimeStorage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::{
        async_job::AsyncJob, obj::LoadedObj, resizable_buffer::ResizableBuffer,
        wgpu_error_scope::WgpuErrorScope,
    },
};

pub mod vertex_buffer;

pub struct ModelCreationContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub file_storage: &'a FileStorage,
    pub runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    pub mtl_dependencies: &'a mut SecondaryMap<ModelId, Vec<FilePath>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    label: String,
    #[serde(default)]
    source: Option<FilePath>,
    material_bind_group_ids: Vec<Option<BindGroupId>>,
    mesh_material_selections: Vec<MeshMaterialSelection>,
    vertex_buffer_spec: VertexBufferSpec,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct ModelRuntime {
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
}

#[derive(Default)]
pub enum ModelJob {
    #[default]
    Start,
    Loading(AsyncJob<AppResult<(ModelRuntime, Vec<FilePath>)>>),
}

pub struct Mesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    texture_coords: Vec<[f32; 2]>,
    tangents: Vec<[f32; 3]>,
    bitangents: Vec<[f32; 3]>,
    indices: Vec<u32>,
    material_index: Option<usize>,
    vertex_buffer: ResizableBuffer,
    index_buffer: ResizableBuffer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MeshMaterialSelection {
    FromSource,
    Material(Option<usize>),
}

impl Default for MeshMaterialSelection {
    fn default() -> Self {
        Self::FromSource
    }
}

pub struct Material {
    label: String,
    texture_paths: Vec<String>,
}

impl Model {
    pub fn new(label: impl Into<String>, source: FilePath) -> Self {
        Self {
            label: label.into(),
            source: Some(source),
            material_bind_group_ids: Vec::new(),
            mesh_material_selections: Vec::new(),
            vertex_buffer_spec: VertexBufferSpec::new(),
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn label() -> &str;
        pub fn source() -> Option<&FilePath>;
        pub fn material_bind_group_ids() -> &[Option<BindGroupId>];
        pub fn mesh_material_selections() -> &[MeshMaterialSelection];
        pub fn vertex_buffer_spec() -> &VertexBufferSpec;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_label(label: String);
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_source(source: Option<FilePath>);
        pub fn set_material_bind_group_ids(material_bind_group_ids: Vec<Option<BindGroupId>>);
        pub fn set_mesh_material_selections(mesh_material_selections: Vec<MeshMaterialSelection>);
        pub fn set_vertex_buffer_spec(vertex_buffer_spec: VertexBufferSpec);
    }

    pub fn material_bind_group_id(&self, material_index: usize) -> Option<BindGroupId> {
        self.material_bind_group_ids
            .get(material_index)
            .copied()
            .flatten()
    }

    pub fn get_material_bind_group_ids(&self) -> Vec<BindGroupId> {
        self.material_bind_group_ids
            .iter()
            .filter_map(|a| *a)
            .collect()
    }

    pub fn mesh_material_selection(&self, mesh_index: usize) -> MeshMaterialSelection {
        self.mesh_material_selections
            .get(mesh_index)
            .cloned()
            .unwrap_or_default()
    }

    pub fn selected_material_index(&self, mesh_index: usize, mesh: &Mesh) -> Option<usize> {
        match self.mesh_material_selection(mesh_index) {
            MeshMaterialSelection::FromSource => mesh.material_index(),
            MeshMaterialSelection::Material(material_index) => material_index,
        }
    }

    fn validate_all_material_bind_group_layouts_are_the_same(
        &self,
        bind_group_ids: &[BindGroupId],
        runtime_bind_groups: &RuntimeStorage<BindGroup>,
    ) -> AppResult<Option<()>> {
        let mut first = None;

        for (i, id) in bind_group_ids.into_iter().copied().enumerate() {
            let Some(bind_group) = runtime_bind_groups.get_init(id)? else {
                // Bind groups aren't ready yet
                return Ok(None);
            };

            let layout_entries = bind_group.layout_entries();

            let Some((first_i, _, first_layout_entries)) = first else {
                first = Some((i, id, layout_entries));
                continue;
            };

            if layout_entries != first_layout_entries {
                return Err(AppError::ModelMaterialBindGroupLayoutMismatch {
                    model_label: self.label.clone(),
                    expected_material_index: first_i,
                    material_index: i,
                });
            }
        }

        Ok(Some(()))
    }
}

impl Creatable for Model {
    fn create(label: String) -> Self {
        Self {
            label,
            source: None,
            material_bind_group_ids: Vec::new(),
            mesh_material_selections: Vec::new(),
            vertex_buffer_spec: VertexBufferSpec::new(),
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }
}

impl ProjectResource for Model {
    type Id = ModelId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for Model {
    type Context<'a> = ModelCreationContext<'a>;
    type Runtime = ModelRuntime;
    type Job = ModelJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            ModelJob::Start => {
                let source = self
                    .source
                    .clone()
                    .ok_or(AppError::uninit_field("Source"))?;
                let vertex_buffer_spec = self.vertex_buffer_spec.clone();
                let device = ctx.device.clone();

                // This is required because the render pipeline expects all
                // material bind group layouts to be the same
                let validation = self.validate_all_material_bind_group_layouts_are_the_same(
                    &self.get_material_bind_group_ids(),
                    ctx.runtime_bind_groups,
                )?;

                if validation.is_none() {
                    // Some bind group is yet to be synced, wait for it to finish
                    return Ok(SyncOutcome::Pending(ModelJob::Start));
                };

                let job = ModelJob::Loading(AsyncJob::new(ModelRuntime::load_from_obj_file(
                    source.clone(),
                    ctx.file_storage,
                    vertex_buffer_spec,
                    device,
                )));

                self.sync(id, ctx, None, job)
            }
            ModelJob::Loading(mut future) => match future.try_resolve() {
                Poll::Ready(result) => {
                    let (runtime, mtl_dependencies) = result?;
                    ctx.mtl_dependencies.insert(id, mtl_dependencies);
                    Ok(SyncOutcome::Changed(runtime))
                }
                Poll::Pending => Ok(SyncOutcome::Pending(ModelJob::Loading(future))),
            },
        }
    }

    fn needs_rebuild(&self, id: Self::Id, ctx: &Self::Context<'_>, tracker: &SyncTracker) -> bool {
        let source_changed = self
            .source
            .as_ref()
            .is_some_and(|source| tracker.file_changed(source));

        let mtl_changed = ctx
            .mtl_dependencies
            .get(id)
            .is_some_and(|dependencies| dependencies.iter().any(|path| tracker.file_changed(path)));

        source_changed || mtl_changed
    }
}

impl ModelRuntime {
    pub fn load_from_obj_file(
        source: FilePath,
        file_storage: &FileStorage,
        vertex_buffer_spec: VertexBufferSpec,
        device: wgpu::Device,
    ) -> AsyncJob<AppResult<(Self, Vec<FilePath>)>> {
        let file_system = file_storage.file_system.clone();
        AsyncJob::new(async move {
            let LoadedObj {
                models,
                materials: obj_materials,
                mtl_dependencies,
            } = crate::utils::obj::load_obj(source, file_system).await?;
            let materials = obj_materials.into_iter().map(Into::into).collect();

            let scope = WgpuErrorScope::push(&device);
            let meshes = models
                .into_iter()
                .enumerate()
                .map(|(i, m)| Mesh::new_from_obj(m, i, &vertex_buffer_spec, &device))
                .collect::<AppResult<Vec<_>>>()?;
            scope.pop().await?;

            let runtime = Self { meshes, materials };

            Ok((runtime, mtl_dependencies))
        })
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    pub fn get_material(&self, material_id: usize) -> Option<&Material> {
        self.materials.get(material_id)
    }
}

impl From<tobj::Material> for Material {
    fn from(material: tobj::Material) -> Self {
        let label = material.name;
        let texture_paths = [
            material.ambient_texture,
            material.diffuse_texture,
            material.normal_texture,
            material.specular_texture,
            material.shininess_texture,
            material.dissolve_texture,
        ]
        .into_iter()
        .filter_map(|tex| tex)
        .collect();

        Material {
            label,
            texture_paths,
        }
    }
}

impl Mesh {
    pub fn new_from_obj(
        model: tobj::Model,
        mesh_index: usize,
        vertex_buffer_spec: &VertexBufferSpec,
        device: &wgpu::Device,
    ) -> AppResult<Self> {
        let name = format!("Mesh {} {mesh_index}", model.name);
        let (positions, _) = model.mesh.positions.as_chunks();
        let (normals, _) = model.mesh.normals.as_chunks();
        let (texture_coords, _) = model.mesh.texcoords.as_chunks();

        let indices = model.mesh.indices;

        let (tangents, bitangents) = crate::utils::obj::calculate_tangents_and_bitangents(
            positions,
            texture_coords,
            &indices,
        );

        let vertex_buffer_contents = vertex_buffer_spec.compute_vertex_contents(
            positions,
            normals,
            texture_coords,
            &tangents,
            &bitangents,
        );

        let vertex_buffer = ResizableBuffer::new(
            device,
            format!("{name} Vertex Buffer"),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(&vertex_buffer_contents),
        );

        let index_buffer = ResizableBuffer::new(
            device,
            format!("{name} Index Buffer"),
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&indices),
        );

        Ok(Self {
            positions: positions.to_vec(),
            normals: normals.to_vec(),
            texture_coords: texture_coords.to_vec(),
            tangents,
            bitangents,
            indices,
            material_index: model.mesh.material_id,
            vertex_buffer,
            index_buffer,
        })
    }

    pub fn positions(&self) -> &[[f32; 3]] {
        &self.positions
    }

    pub fn normals(&self) -> &[[f32; 3]] {
        &self.normals
    }

    pub fn texture_coords(&self) -> &[[f32; 2]] {
        &self.texture_coords
    }

    pub fn tangents(&self) -> &[[f32; 3]] {
        &self.tangents
    }

    pub fn bitangents(&self) -> &[[f32; 3]] {
        &self.bitangents
    }

    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    pub fn material_index(&self) -> Option<usize> {
        self.material_index
    }

    pub fn vertex_buffer(&self) -> &ResizableBuffer {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &ResizableBuffer {
        &self.index_buffer
    }
}

impl Material {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn texture_paths(&self) -> &[String] {
        &self.texture_paths
    }
}
