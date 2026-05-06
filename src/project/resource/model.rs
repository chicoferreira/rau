use std::task::Poll;

use futures_lite::io::{BufReader, Cursor};
use wgpu::BindGroupLayout;

use crate::{
    error::{AppError, AppResult},
    file::{
        file_storage::FileStorage,
        file_system::{FileSystem, FileSystemTrait},
    },
    project::{
        BindGroupId, ModelId, ProjectResource,
        paths::FilePath,
        resource::{
            bindgroup::BindGroup,
            model::vertex_buffer::{VertexBufferField, VertexBufferSpec},
        },
        storage::RuntimeStorage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils::{
        async_job::AsyncJob, resizable_buffer::ResizableBuffer, wgpu_error_scope::WgpuErrorScope,
    },
};

pub mod vertex_buffer;

pub struct ModelCreationContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub file_storage: &'a FileStorage,
}

pub struct Model {
    pub label: String,
    source: FilePath,
    material_bind_group_ids: Vec<Option<BindGroupId>>,
    mesh_material_selections: Vec<MeshMaterialSelection>,
    vertex_buffer_spec: VertexBufferSpec,
    revision: Revision,
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

#[derive(Debug, Clone, PartialEq)]
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
            source,
            material_bind_group_ids: Vec::new(),
            mesh_material_selections: Vec::new(),
            vertex_buffer_spec: VertexBufferSpec::new(),
            revision: Revision::default(),
        }
    }

    pub fn source(&self) -> &FilePath {
        &self.source
    }

    pub fn set_source(&mut self, source: FilePath) {
        if self.source != source {
            self.source = source;
            self.revision.increase();
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
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
            self.revision.increase();
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
            self.revision.increase();
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
        self.revision.increase();
    }

    pub fn remove_vertex_buffer_field(&mut self, index: usize) {
        let fields = &mut self.vertex_buffer_spec.fields;
        if index < fields.len() {
            fields.remove(index);
            self.revision.increase();
        }
    }

    pub fn set_vertex_buffer_field(&mut self, index: usize, field: VertexBufferField) {
        if let Some(f) = self.vertex_buffer_spec.fields.get_mut(index) {
            *f = field;
            self.revision.increase();
        }
    }

    pub fn reorder_vertex_buffer_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        self.vertex_buffer_spec.reorder_field(from, to);
        self.revision.increase();
    }
}

impl ProjectResource for Model {
    type Id = ModelId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for Model {
    type Context<'a> = ModelCreationContext<'a>;
    type Runtime = ModelRuntime;
    type Job = ModelJob;

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            ModelJob::Start => {
                let source = self.source.clone();
                let file_system = ctx.file_storage.file_system.clone();
                let vertex_buffer_spec = self.vertex_buffer_spec.clone();
                let device = ctx.device.clone();

                Ok(SyncOutcome::Pending(ModelJob::Loading(AsyncJob::new(
                    ModelRuntime::load_from_obj_file(
                        source,
                        file_system,
                        vertex_buffer_spec,
                        device,
                    ),
                ))))
            }
            ModelJob::Loading(mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(SyncOutcome::Changed),
                Poll::Pending => Ok(SyncOutcome::Pending(ModelJob::Loading(future))),
            },
        }
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        tracker.file_changed(&self.source)
    }
}

impl ModelRuntime {
    pub async fn load_from_obj_file(
        source: FilePath,
        file_system: FileSystem, // TODO: use file storage for read cache
        vertex_buffer_spec: VertexBufferSpec,
        device: wgpu::Device,
    ) -> AppResult<Self> {
        let obj_bytes = file_system.read(&source).await?;

        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let obj_reader = BufReader::new(Cursor::new(obj_bytes));
        let (models, obj_materials) =
            tobj::futures::load_obj_buf(obj_reader, &load_options, move |material_path| {
                let material_path = material_path.to_string_lossy().to_string();
                // TODO: don't use this load_obj_buf function. load mtls manually so we can return errors and avoid the need to clone file_system
                let material_path = FilePath::from_str(material_path).unwrap();
                let material_path = source
                    .parent()
                    .map(|parent| parent.join_path(&material_path))
                    .unwrap_or_else(|| material_path);

                let file_system = file_system.clone();

                async move {
                    let mat = file_system
                        .read(&material_path)
                        .await
                        .map_err(|_| tobj::LoadError::OpenFileFailed)?;
                    let reader = BufReader::new(Cursor::new(mat));
                    tobj::futures::load_mtl_buf(reader).await
                }
            })
            .await?;

        let scope = WgpuErrorScope::push(&device);
        let meshes = models
            .into_iter()
            .map(|m| Mesh::new_from_obj(m, &vertex_buffer_spec, &device))
            .collect::<AppResult<Vec<_>>>()?;

        let materials = obj_materials?.into_iter().map(Into::into).collect();
        let runtime = Self { meshes, materials };
        scope.pop().await?;

        Ok(runtime)
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
    ) -> AppResult<&'a BindGroupLayout> {
        let mesh = self.meshes().first().ok_or(AppError::UninitializedFields)?;
        let m_index = model
            .selected_material_index(0, mesh)
            .ok_or(AppError::UninitializedFields)?;
        self.get_material(m_index)
            .ok_or(AppError::UninitializedFields)?;
        let bind_group_id = model
            .material_bind_group_id(m_index)
            .ok_or(AppError::UninitializedFields)?;
        let bind_group = runtime_bind_groups.get_init(bind_group_id)?;
        Ok(bind_group.inner_layout())
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
        vertex_buffer_spec: &VertexBufferSpec,
        device: &wgpu::Device,
    ) -> AppResult<Self> {
        let (positions, _) = model.mesh.positions.as_chunks();
        let (normals, _) = model.mesh.normals.as_chunks();
        let (texture_coords, _) = model.mesh.texcoords.as_chunks();

        let indices = model.mesh.indices;

        let (tangents, bitangents) =
            Self::calculate_tangents_and_bitangents(positions, texture_coords, &indices);

        let vertex_buffer_contents = Self::calculate_compute_vertex_contents(
            positions,
            normals,
            texture_coords,
            &tangents,
            &bitangents,
            vertex_buffer_spec,
        );

        let vertex_buffer = ResizableBuffer::new(
            device,
            "TODO: add vertex buffer name",
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            bytemuck::cast_slice(&vertex_buffer_contents),
        );

        let index_buffer = ResizableBuffer::new(
            device,
            "TODO: add index buffer name",
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

    fn calculate_tangents_and_bitangents(
        positions: &[[f32; 3]],
        texture_coords: &[[f32; 2]],
        indices: &[u32],
    ) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
        use cgmath::{InnerSpace, Vector2, Vector3};

        let vertex_count = positions.len();

        let mut tangents = vec![[0.0f32; 3]; vertex_count];
        let mut bitangents = vec![[0.0f32; 3]; vertex_count];
        let mut triangles_included = vec![0u32; vertex_count];

        let (triangles, _) = indices.as_chunks();
        for [i0, i1, i2] in triangles {
            let (i0, i1, i2) = (*i0 as usize, *i1 as usize, *i2 as usize);
            if i0 >= vertex_count || i1 >= vertex_count || i2 >= vertex_count {
                continue;
            }

            let p0: Vector3<_> = positions.get(i0).copied().unwrap_or([0.0, 0.0, 0.0]).into();
            let p1: Vector3<_> = positions.get(i1).copied().unwrap_or([0.0, 0.0, 0.0]).into();
            let p2: Vector3<_> = positions.get(i2).copied().unwrap_or([0.0, 0.0, 0.0]).into();
            let uv0: Vector2<_> = texture_coords.get(i0).copied().unwrap_or([0.0, 0.0]).into();
            let uv1: Vector2<_> = texture_coords.get(i1).copied().unwrap_or([0.0, 0.0]).into();
            let uv2: Vector2<_> = texture_coords.get(i2).copied().unwrap_or([0.0, 0.0]).into();

            let dp1 = p1 - p0;
            let dp2 = p2 - p0;
            let duv1 = uv1 - uv0;
            let duv2 = uv2 - uv0;

            let denom = duv1.x * duv2.y - duv1.y * duv2.x;
            if denom.abs() < 1.0e-8 {
                continue;
            }
            let r = 1.0 / denom;

            let tangent = (dp1 * duv2.y - dp2 * duv1.y) * r;
            let bitangent = (dp2 * duv1.x - dp1 * duv2.x) * r;

            tangents[i0] = (Vector3::from(tangents[i0]) + tangent).into();
            tangents[i1] = (Vector3::from(tangents[i1]) + tangent).into();
            tangents[i2] = (Vector3::from(tangents[i2]) + tangent).into();

            bitangents[i0] = (Vector3::from(bitangents[i0]) + bitangent).into();
            bitangents[i1] = (Vector3::from(bitangents[i1]) + bitangent).into();
            bitangents[i2] = (Vector3::from(bitangents[i2]) + bitangent).into();

            triangles_included[i0] += 1;
            triangles_included[i1] += 1;
            triangles_included[i2] += 1;
        }

        for i in 0..vertex_count {
            let n = triangles_included[i];
            if n <= 0 {
                continue;
            }
            let denom = 1.0 / n as f32;

            let t = Vector3::from(tangents[i]) * denom;
            let b = Vector3::from(bitangents[i]) * denom;

            tangents[i] = if t.magnitude2() > 0.0 {
                t.normalize().into()
            } else {
                [0.0, 0.0, 0.0]
            };
            bitangents[i] = if b.magnitude2() > 0.0 {
                b.normalize().into()
            } else {
                [0.0, 0.0, 0.0]
            };
        }

        (tangents, bitangents)
    }

    fn calculate_compute_vertex_contents(
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        texture_coords: &[[f32; 2]],
        tangents: &[[f32; 3]],
        bitangents: &[[f32; 3]],
        vertex_buffer_spec: &VertexBufferSpec,
    ) -> Vec<f32> {
        let vertex_count = positions.len();

        let stride: usize = vertex_buffer_spec
            .fields
            .iter()
            .map(|f| f.vertex_format().size() as usize / std::mem::size_of::<f32>())
            .sum();

        let mut result = Vec::with_capacity(vertex_count * stride);

        for i in 0..vertex_count {
            let p = positions.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let n = normals.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let uv = texture_coords.get(i).unwrap_or(&[0.0, 0.0]);
            let t = tangents.get(i).unwrap_or(&[0.0, 0.0, 0.0]);
            let b = bitangents.get(i).unwrap_or(&[0.0, 0.0, 0.0]);

            for f in &vertex_buffer_spec.fields {
                let value: &[f32] = match f {
                    vertex_buffer::VertexBufferField::Position => p,
                    vertex_buffer::VertexBufferField::TextureCoordinates => uv,
                    vertex_buffer::VertexBufferField::Normal => n,
                    vertex_buffer::VertexBufferField::Tangent => t,
                    vertex_buffer::VertexBufferField::Bitangent => b,
                };
                result.extend_from_slice(value);
            }
        }

        result
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
