use std::task::Poll;

use serde::{Deserialize, Serialize};
use wgpu::BindGroupLayout;

use crate::{
    error::{AppError, AppResult},
    file::{file_storage::FileStorage, file_system::ProjectFileSystemTrait},
    project::{
        BindGroupId, Creatable, ModelId, ProjectResource,
        paths::FilePath,
        resource::{
            bindgroup::BindGroup,
            model::vertex_buffer::{VertexBufferField, VertexBufferSpec},
        },
        storage::RuntimeStorage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
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
    Loading(AsyncJob<AppResult<ModelRuntime>>),
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

    pub fn source(&self) -> Option<&FilePath> {
        self.source.as_ref()
    }

    pub fn set_source(&mut self, source: Option<FilePath>) {
        if self.source != source {
            self.source = source;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn set_label(&mut self, label: String) {
        if self.label != label {
            self.label = label;
            self.project_revision.increase();
        }
    }

    pub fn material_bind_group_id(&self, material_index: usize) -> Option<BindGroupId> {
        self.material_bind_group_ids
            .get(material_index)
            .copied()
            .flatten()
    }

    pub fn set_material_bind_group_id(
        &mut self,
        material_index: usize,
        bind_group_id: Option<BindGroupId>,
    ) {
        if self.material_bind_group_ids.len() <= material_index {
            self.material_bind_group_ids
                .resize(material_index + 1, None);
        }

        if self.material_bind_group_ids[material_index] != bind_group_id {
            self.material_bind_group_ids[material_index] = bind_group_id;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn mesh_material_selection(&self, mesh_index: usize) -> MeshMaterialSelection {
        self.mesh_material_selections
            .get(mesh_index)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_mesh_material_selection(
        &mut self,
        mesh_index: usize,
        selection: MeshMaterialSelection,
    ) {
        if self.mesh_material_selections.len() <= mesh_index {
            self.mesh_material_selections
                .resize(mesh_index + 1, MeshMaterialSelection::default());
        }

        if self.mesh_material_selections[mesh_index] != selection {
            self.mesh_material_selections[mesh_index] = selection;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn selected_material_index(&self, mesh_index: usize, mesh: &Mesh) -> Option<usize> {
        match self.mesh_material_selection(mesh_index) {
            MeshMaterialSelection::FromSource => mesh.material_index(),
            MeshMaterialSelection::Material(material_index) => material_index,
        }
    }

    pub fn vertex_buffer_spec(&self) -> &VertexBufferSpec {
        &self.vertex_buffer_spec
    }

    pub fn add_vertex_buffer_field(&mut self, field: VertexBufferField) {
        self.vertex_buffer_spec.fields.push(field);
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn remove_vertex_buffer_field(&mut self, index: usize) {
        let fields = &mut self.vertex_buffer_spec.fields;
        if index < fields.len() {
            fields.remove(index);
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn set_vertex_buffer_field(&mut self, index: usize, field: VertexBufferField) {
        if let Some(f) = self.vertex_buffer_spec.fields.get_mut(index) {
            *f = field;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }

    pub fn reorder_vertex_buffer_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        self.vertex_buffer_spec.reorder_field(from, to);
        self.runtime_revision.increase();
        self.project_revision.increase();
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
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            ModelJob::Start => {
                let source = self.source.clone().ok_or(AppError::UninitializedFields)?;
                let vertex_buffer_spec = self.vertex_buffer_spec.clone();
                let device = ctx.device.clone();

                self.sync(
                    ctx,
                    None,
                    ModelJob::Loading(AsyncJob::new(ModelRuntime::load_from_obj_file(
                        source,
                        ctx.file_storage,
                        vertex_buffer_spec,
                        device,
                    ))),
                )
            }
            ModelJob::Loading(mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(SyncOutcome::Changed),
                Poll::Pending => Ok(SyncOutcome::Pending(ModelJob::Loading(future))),
            },
        }
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.source
            .as_ref()
            .is_some_and(|source| tracker.file_changed(source))
    }
}

impl ModelRuntime {
    pub fn load_from_obj_file(
        source: FilePath,
        file_storage: &FileStorage,
        vertex_buffer_spec: VertexBufferSpec,
        device: wgpu::Device,
    ) -> AsyncJob<AppResult<Self>> {
        let file_system = file_storage.file_system.clone();
        AsyncJob::new(async move {
            let obj_bytes = file_system.read(&source).await?;

            let LoadedObj { models, mtl_paths } = crate::utils::obj::load_obj(&obj_bytes, &source)?;

            let mut materials = vec![];
            for mtl_path in mtl_paths {
                let mtl_bytes = file_system.read(&mtl_path).await?;
                let mtl_materials = crate::utils::obj::load_mtl(&mtl_bytes)?
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>();

                materials.extend(mtl_materials);
            }

            let scope = WgpuErrorScope::push(&device);
            let meshes = models
                .into_iter()
                .enumerate()
                .map(|(i, m)| Mesh::new_from_obj(m, i, &vertex_buffer_spec, &device))
                .collect::<AppResult<Vec<_>>>()?;
            scope.pop().await?;

            let runtime = Self { meshes, materials };

            Ok(runtime)
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

    /// Returns the bind group layout of the first mesh.
    /// This assumes all the meshes have the same bind group layout;
    /// Otherwise the render pipeline will throw an error during rendering.
    pub fn get_bind_group_layout<'a>(
        &self,
        model: &Model,
        runtime_bind_groups: &'a RuntimeStorage<BindGroup>,
    ) -> AppResult<Option<&'a BindGroupLayout>> {
        let mesh = self.meshes().first().ok_or(AppError::UninitializedFields)?;
        let m_index = model
            .selected_material_index(0, mesh)
            .ok_or(AppError::UninitializedFields)?;
        self.get_material(m_index)
            .ok_or(AppError::UninitializedFields)?;
        let bind_group_id = model
            .material_bind_group_id(m_index)
            .ok_or(AppError::UninitializedFields)?;
        let Some(bind_group) = runtime_bind_groups.get_init(bind_group_id)? else {
            return Ok(None);
        };
        Ok(Some(bind_group.inner_layout()))
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
