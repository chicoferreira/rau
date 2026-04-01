use std::{io::BufReader, path::Path};

use crate::{
    error::{AppError, AppResult},
    project::model::vertex_buffer::VertexBufferSpec,
    resources::load_binary,
};

pub mod vertex_buffer;

pub struct Model {
    pub label: String,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub vertex_buffer_spec: VertexBufferSpec,
}

pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub texture_coords: Vec<[f32; 2]>,
    pub tangents: Vec<[f32; 3]>,
    pub bitangents: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub material_id: Option<usize>,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

pub struct Material {
    pub label: String,
    pub texture_paths: Vec<String>,
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
            .collect();
        let materials = obj_materials?.into_iter().map(Into::into).collect();

        Ok(Model {
            label,
            meshes,
            materials,
            vertex_buffer_spec,
        })
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
    ) -> Self {
        let (positions, _) = model.mesh.positions.as_chunks();
        let (normals, _) = model.mesh.normals.as_chunks();
        let (texture_coords, _) = model.mesh.texcoords.as_chunks();

        let indices = model.mesh.indices;

        let (tangents, bitangents) =
            Self::calculate_tangents_and_bitangents(positions, texture_coords, &indices);

        let (vertex_buffer, index_buffer) = Self::create_vertex_and_index_buffers(
            positions,
            normals,
            texture_coords,
            &tangents,
            &bitangents,
            &indices,
            vertex_buffer_spec,
            device,
        );

        Self {
            positions: positions.to_vec(),
            normals: normals.to_vec(),
            texture_coords: texture_coords.to_vec(),
            tangents,
            bitangents,
            indices,
            material_id: model.mesh.material_id,
            vertex_buffer,
            index_buffer,
        }
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

    fn create_vertex_and_index_buffers(
        positions: &[[f32; 3]],
        normals: &[[f32; 3]],
        texture_coords: &[[f32; 2]],
        tangents: &[[f32; 3]],
        bitangents: &[[f32; 3]],
        indices: &[u32],
        vertex_buffer_spec: &VertexBufferSpec,
        device: &wgpu::Device,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        use wgpu::util::DeviceExt;

        let vertex_count = positions.len();

        let stride: usize = vertex_buffer_spec
            .fields
            .iter()
            .map(|f| f.vertex_format().size() as usize / std::mem::size_of::<f32>())
            .sum();

        let mut interleaved = Vec::with_capacity(vertex_count * stride);

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
                interleaved.extend_from_slice(value);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Project Model Vertex Buffer"),
            contents: bytemuck::cast_slice(&interleaved),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Project Model Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer)
    }
}
