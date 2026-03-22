use std::io::{BufReader, Cursor};

use anyhow::Context;
use wgpu::util::DeviceExt;

use crate::{
    error::{AppError, AppResult},
    model,
    project::{
        self, Project, SamplerId, TextureViewId,
        bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
        texture::{Texture, TextureCreationContext, TextureSource},
        texture_view::{TextureView, TextureViewCreationContext},
    },
    ui::renderer::EguiRenderer,
};

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    load_binary(file_name)
        .await?
        .try_into()
        .context("Failed to parse UTF-8 string")
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    let data = {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let mut origin = location.origin().unwrap();
        if !origin.ends_with("res") {
            origin = format!("{}/res", origin);
        }
        let base = reqwest::Url::parse(&format!("{}/", origin)).unwrap();
        let url = base.join(file_name).unwrap();
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let data = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}

pub async fn load_texture(
    ctx: &TextureCreationContext<'_>,
    file_name: &str,
    is_normal_map: bool,
) -> AppResult<Texture> {
    let data = load_binary(file_name)
        .await
        .map_err(AppError::FileLoadError)?;
    let img = image::load_from_memory(&data)?;

    let format = match is_normal_map {
        false => wgpu::TextureFormat::Rgba8UnormSrgb,
        true => wgpu::TextureFormat::Rgba8Unorm,
    };

    let source = TextureSource::Image(img);

    Ok(Texture::new(
        ctx,
        file_name.to_string(),
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        source,
    )?)
}

fn create_material_bind_group(
    project: &Project,
    device: &wgpu::Device,
    label: String,
    diffuse_texture_view_id: TextureViewId,
    normal_texture_view_id: TextureViewId,
    sampler_id: SamplerId,
) -> AppResult<BindGroup> {
    let entries = vec![
        // TODO: Remove the duplicated sampler
        BindGroupEntry::new(BindGroupResource::Texture {
            texture_view_id: Some(diffuse_texture_view_id),
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        }),
        BindGroupEntry::new(BindGroupResource::Sampler {
            sampler_id: Some(sampler_id),
            sampler_binding_type: wgpu::SamplerBindingType::Filtering,
        }),
        BindGroupEntry::new(BindGroupResource::Texture {
            texture_view_id: Some(normal_texture_view_id),
            view_dimension: wgpu::TextureViewDimension::D2,
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
        }),
        BindGroupEntry::new(BindGroupResource::Sampler {
            sampler_id: Some(sampler_id),
            sampler_binding_type: wgpu::SamplerBindingType::Filtering,
        }),
    ];

    BindGroup::new(project, device, label, entries)
}

pub async fn load_model(
    project: &mut project::Project,
    file_name: &str,
    egui_renderer: &mut EguiRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sampler_id: SamplerId,
) -> AppResult<model::Model> {
    let obj_text = load_string(file_name)
        .await
        .map_err(AppError::FileLoadError)?;
    let obj_bytes = obj_text.as_bytes();

    let (models, obj_materials) = tobj::futures::load_obj_buf(
        obj_bytes,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p.to_string_lossy()).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();

    let ctx = TextureCreationContext {
        dimensions: &project.dimensions,
        device,
        queue,
    };

    for m in obj_materials? {
        let file_name = m.diffuse_texture.unwrap();
        let diffuse_texture = load_texture(&ctx, &file_name, false).await?;

        let diffuse_texture_id = project.textures.register(diffuse_texture);
        let diffuse_texture_view_id = project.texture_views.register(TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            file_name.clone(),
            diffuse_texture_id,
            None,
            None,
        )?);

        let normal_texture = load_texture(&ctx, &m.normal_texture.unwrap(), true).await?;

        let normal_texture_id = project.textures.register(normal_texture);
        let normal_texture_view_id = project.texture_views.register(TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            file_name.clone(),
            normal_texture_id,
            None,
            None,
        )?);

        let material_bind_group = create_material_bind_group(
            project,
            device,
            file_name.clone(),
            diffuse_texture_view_id,
            normal_texture_view_id,
            sampler_id,
        )?;

        let material_bind_group_id = project.bind_groups.register(material_bind_group);

        materials.push(material_bind_group_id);
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let mut vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                    tangent: [0.0; 3],
                    bitangent: [0.0; 3],
                })
                .collect::<Vec<_>>();

            let indices = &m.mesh.indices;
            let mut triangles_included = vec![0; vertices.len()];

            for c in indices.chunks(3) {
                let v0 = vertices[c[0] as usize];
                let v1 = vertices[c[1] as usize];
                let v2 = vertices[c[2] as usize];

                let pos0: cgmath::Vector3<_> = v0.position.into();
                let pos1: cgmath::Vector3<_> = v1.position.into();
                let pos2: cgmath::Vector3<_> = v2.position.into();

                let uv0: cgmath::Vector2<_> = v0.tex_coords.into();
                let uv1: cgmath::Vector2<_> = v1.tex_coords.into();
                let uv2: cgmath::Vector2<_> = v2.tex_coords.into();

                let delta_pos1 = pos1 - pos0;
                let delta_pos2 = pos2 - pos0;

                let delta_uv1 = uv1 - uv0;
                let delta_uv2 = uv2 - uv0;

                let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
                let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
                let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

                vertices[c[0] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[0] as usize].tangent)).into();
                vertices[c[1] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[1] as usize].tangent)).into();
                vertices[c[2] as usize].tangent =
                    (tangent + cgmath::Vector3::from(vertices[c[2] as usize].tangent)).into();
                vertices[c[0] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[0] as usize].bitangent)).into();
                vertices[c[1] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[1] as usize].bitangent)).into();
                vertices[c[2] as usize].bitangent =
                    (bitangent + cgmath::Vector3::from(vertices[c[2] as usize].bitangent)).into();

                triangles_included[c[0] as usize] += 1;
                triangles_included[c[1] as usize] += 1;
                triangles_included[c[2] as usize] += 1;
            }

            for (i, n) in triangles_included.into_iter().enumerate() {
                let denom = 1.0 / n as f32;
                let v = &mut vertices[i];
                v.tangent = (cgmath::Vector3::from(v.tangent) * denom).into();
                v.bitangent = (cgmath::Vector3::from(v.bitangent) * denom).into();
            }

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

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material_bind_group_id: materials[m.mesh.material_id.unwrap_or(0)],
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes })
}
