use std::{io::BufReader, path::Path};

use crate::{
    error::{AppError, AppResult},
    project::{
        BindGroupId, ProjectResource,
        model::vertex_buffer::{VertexBufferField, VertexBufferSpec},
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
    },
    resources::load_binary,
    utils::resizable_buffer::{ChangeResult, ResizableBuffer},
};

pub mod vertex_buffer;

pub struct ModelCreationContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct Model {
    pub label: String,
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    vertex_buffer_spec: VertexBufferSpec,
    dirty: bool,
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

pub struct Material {
    label: String,
    texture_paths: Vec<String>,
    bind_group_id: Option<BindGroupId>,
}

impl Model {
    pub async fn load_from_obj_file(
        label: String,
        file: impl AsRef<Path>,
        device: &wgpu::Device,
    ) -> AppResult<Self> {
        let obj_bytes = load_binary(file).await.map_err(AppError::FileLoadError)?;

        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (models, obj_materials) =
            tobj::futures::load_obj_buf(obj_bytes.as_ref(), &load_options, |p| async move {
                let mat = load_binary(&p).await;
                let mat = mat.map_err(|_| tobj::LoadError::OpenFileFailed)?;
                let mat: &[u8] = mat.as_ref();
                let mut reader = BufReader::new(mat);
                tobj::load_mtl_buf(&mut reader)
            })
            .await?;

        let vertex_buffer_spec = VertexBufferSpec::new();

        let meshes = models
            .into_iter()
            .map(|m| Mesh::new_from_obj(m, &vertex_buffer_spec, device))
            .collect::<AppResult<Vec<_>>>()?;

        let materials = obj_materials?.into_iter().map(Into::into).collect();

        Ok(Model {
            label,
            meshes,
            materials,
            vertex_buffer_spec,
            dirty: false,
        })
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    pub fn materials(&self) -> &[Material] {
        &self.materials
    }

    pub fn materials_mut(&mut self) -> &mut [Material] {
        &mut self.materials
    }

    pub fn get_material(&self, material_id: usize) -> Option<&Material> {
        self.materials.get(material_id)
    }

    pub fn vertex_buffer_spec(&self) -> &VertexBufferSpec {
        &self.vertex_buffer_spec
    }

    pub fn add_vertex_buffer_field(&mut self, field: VertexBufferField) {
        self.vertex_buffer_spec.fields.push(field);
        self.dirty = true;
    }

    pub fn remove_vertex_buffer_field(&mut self, index: usize) {
        let fields = &mut self.vertex_buffer_spec.fields;
        if index < fields.len() {
            fields.remove(index);
            self.dirty = true;
        }
    }

    pub fn set_vertex_buffer_field(&mut self, index: usize, field: VertexBufferField) {
        if let Some(f) = self.vertex_buffer_spec.fields.get_mut(index) {
            *f = field;
            self.dirty = true;
        }
    }

    pub fn reorder_vertex_buffer_field(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        self.vertex_buffer_spec.reorder_field(from, to);
        self.dirty = true;
    }
}

impl ProjectResource for Model {
    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Model {
    type Context<'a> = ModelCreationContext<'a>;
    type Id = super::ModelId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        if !self.dirty {
            return Ok(None);
        }

        let spec = &self.vertex_buffer_spec;
        let mut any_recreated = false;
        for mesh in &mut self.meshes {
            match mesh.write_vertex_buffer_from_spec(spec, ctx.device, ctx.queue)? {
                ChangeResult::Uploaded => {}
                ChangeResult::Recreated => any_recreated = true,
            }
        }

        self.dirty = false;

        Ok(any_recreated.then_some(ProjectEvent::ModelVertexBufferRecreated(id)))
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
            bind_group_id: None,
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
        )?;

        let index_buffer = ResizableBuffer::new(
            device,
            "TODO: add index buffer name",
            wgpu::BufferUsages::INDEX,
            bytemuck::cast_slice(&indices),
        )?;

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

    fn write_vertex_buffer_from_spec(
        &mut self,
        vertex_buffer_spec: &VertexBufferSpec,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> AppResult<ChangeResult> {
        let vertex_buffer_contents = Self::calculate_compute_vertex_contents(
            &self.positions,
            &self.normals,
            &self.texture_coords,
            &self.tangents,
            &self.bitangents,
            vertex_buffer_spec,
        );
        self.vertex_buffer.write(
            device,
            queue,
            "TODO: add vertex buffer name",
            bytemuck::cast_slice(&vertex_buffer_contents),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        )
    }
}

impl Material {
    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn texture_paths(&self) -> &[String] {
        &self.texture_paths
    }

    pub fn bind_group_id(&self) -> Option<BindGroupId> {
        self.bind_group_id
    }

    pub fn set_bind_group_id(&mut self, bind_group_id: Option<BindGroupId>) {
        self.bind_group_id = bind_group_id;
    }
}
