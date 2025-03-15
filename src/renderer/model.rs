use crate::file;
use anyhow::Context;
use std::io::BufReader;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32x3,
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
}

pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

pub async fn load_model_from_obj(
    path: impl AsRef<Path>,
    device: &wgpu::Device,
) -> anyhow::Result<Model> {
    let buf = file::load_file_bytes(&path).await?;
    let mut buf = BufReader::new(&buf[..]);
    let (models, materials) = tobj::load_obj_buf(
        &mut buf,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |mat_path| {
            let full_path = if let Some(parent) = path.as_ref().parent() {
                parent.join(mat_path)
            } else {
                mat_path.to_owned()
            };

            tobj::load_mtl(full_path)
        },
    )
    .context("Failed to load obj file")?;

    // TODO: handle loaded materials
    let _materials = materials.context("Failed to load materials")?;

    let file_name = path.as_ref().file_name().unwrap().to_string_lossy();

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| Vertex {
                    position: [
                        get_or_default(&m.mesh.positions, i * 3),
                        get_or_default(&m.mesh.positions, i * 3 + 1),
                        get_or_default(&m.mesh.positions, i * 3 + 2),
                    ],
                    tex_coords: [
                        get_or_default(&m.mesh.texcoords, i * 2),
                        1.0 - get_or_default(&m.mesh.texcoords, i * 2 + 1),
                    ],
                    normal: [
                        get_or_default(&m.mesh.normals, i * 3),
                        get_or_default(&m.mesh.normals, i * 3 + 1),
                        get_or_default(&m.mesh.normals, i * 3 + 2),
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            Mesh {
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
            }
        })
        .collect();

    Ok(Model { meshes })
}

fn get_or_default<T: Copy + Default>(slice: &[T], index: usize) -> T {
    slice.get(index).copied().unwrap_or_default()
}
