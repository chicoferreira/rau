use crate::{
    error::AppResult,
    project::{
        Project, RuntimeProject, ShaderId, TextureId, TextureViewId,
        bindgroup::{BindGroup, BindGroupCreationContext, BindGroupEntry, BindGroupResource},
        shader::ShaderCreationContext,
        sync::SyncTracker,
        texture::{Texture, TextureCreationContext, TextureSource},
        texture_view::{TextureView, TextureViewCreationContext},
    },
    ui::renderer::EguiRenderer,
};

pub fn from_equirectangular_bytes(
    project: &mut Project,
    runtime_project: &mut RuntimeProject,
    recreate_tracker: &mut SyncTracker, // we need this for now to force creation of temporary texture views (remove later)
    egui_renderer: &mut EguiRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    equi_shader_id: ShaderId,
    src_texture: TextureViewId,
    dst_size: u32,
) -> AppResult<TextureId> {
    let mut ctx = ShaderCreationContext { device };
    let shader_runtime = recreate_tracker
        .sync(
            equi_shader_id,
            &mut project.shaders,
            &mut runtime_project.shaders,
            &mut ctx,
            device,
        )?
        .expect("The shader was synced successfully");

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

    recreate_tracker.sync_storage(
        &mut project.textures,
        &mut runtime_project.textures,
        &mut TextureCreationContext {
            dimensions: &project.dimensions,
            device,
            queue,
        },
        device,
    );

    recreate_tracker.sync_storage(
        &mut project.texture_views,
        &mut runtime_project.texture_views,
        &mut TextureViewCreationContext {
            textures: &project.textures,
            textures_runtime: &runtime_project.textures,
            egui_renderer,
            device,
        },
        device,
    );

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

    let bind_group_runtime = recreate_tracker
        .sync(
            bind_group_id,
            &mut project.bind_groups,
            &mut runtime_project.bind_groups,
            &mut BindGroupCreationContext {
                runtime_uniforms: &mut runtime_project.uniforms,
                runtime_texture_views: &mut runtime_project.texture_views,
                runtime_samplers: &mut runtime_project.samplers,
                device,
            },
            device,
        )?
        .expect("this should be created");

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(bind_group_runtime.inner_layout())],
        immediate_size: 0,
    });

    let equirect_to_cubemap = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("equirect_to_cubemap"),
        layout: Some(&pipeline_layout),
        module: shader_runtime.inner(),
        entry_point: Some("compute_equirect_to_cubemap"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: Some("compute shader pass"),
        timestamp_writes: None,
    });

    let num_workgroups = (dst_size + 15) / 16;
    pass.set_pipeline(&equirect_to_cubemap);
    pass.set_bind_group(0, bind_group_runtime.inner(), &[]);
    pass.dispatch_workgroups(num_workgroups, num_workgroups, 6);

    drop(pass);

    queue.submit([encoder.finish()]);

    Ok(dst_texture_id)
}
