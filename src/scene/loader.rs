use crate::{
    error::AppResult,
    project::{
        Project, ShaderId, TextureId, TextureViewId,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            compute_pass::{ComputePass, ComputePassBindGroupEntry},
            texture::{Texture, TextureSource},
            texture_view::TextureView,
        },
    },
};

pub fn from_equirectangular_bytes(
    project: &mut Project,
    equi_shader_id: ShaderId,
    src_texture: TextureViewId,
    dst_size: u32,
) -> AppResult<TextureId> {
    let texture_format = wgpu::TextureFormat::Rgba32Float;

    let dst_texture = Texture::new(
        "Sky Texture",
        texture_format,
        wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST,
        TextureSource::Manual {
            size: wgpu::Extent3d {
                width: dst_size,
                height: dst_size,
                depth_or_array_layers: 6,
            },
        },
    );

    let dst_texture_id = project.textures.register(dst_texture);

    let dst_texture_view = TextureView::new(
        "compute shader destination texture view",
        Some(dst_texture_id),
        None,
        Some(wgpu::TextureViewDimension::D2Array),
    );
    let dst_texture_view_id = project.texture_views.register(dst_texture_view);

    let bind_group = BindGroup::new(
        "compute shader bind group",
        vec![
            BindGroupEntry::new_compute(BindGroupResource::Texture {
                texture_view_id: Some(src_texture),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_compute(BindGroupResource::StorageTexture {
                texture_view_id: Some(dst_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2Array,
                access: wgpu::StorageTextureAccess::WriteOnly,
            }),
        ],
    );

    let bind_group_id = project.bind_groups.register(bind_group);

    let num_workgroups = (dst_size + 15) / 16;
    let compute_pass = ComputePass::new(
        "equirect_to_cube_map",
        vec![ComputePassBindGroupEntry::new(Some(bind_group_id))],
        Some(equi_shader_id),
        num_workgroups,
        num_workgroups,
        6,
    );

    project.compute_passes.register(compute_pass);

    Ok(dst_texture_id)
}
