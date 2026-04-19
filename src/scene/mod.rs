use crate::{
    error::{AppError, AppResult},
    project::{
        self, Project, RuntimeProject, ViewportId,
        bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
        camera::Camera,
        dimension::Dimension,
        model::Model,
        renderpass::{LoadOperation, RenderDraw, RenderPass, RenderPassTarget},
        sampler::{Sampler, SamplerSpec},
        sync::SyncTracker,
        texture::{Texture, TextureSource},
        texture_view::{TextureView, TextureViewFormat},
        uniform::{
            Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
        },
    },
    ui::{self},
    utils::resources::{self, load_texture},
};

mod loader;

pub async fn create_scene(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    size: ui::Size2d,
    project: &mut Project,
    runtime_project: &mut RuntimeProject,
    recreate_tracker: &mut SyncTracker,
    equirectangular_shader_id: project::ShaderId,
    hdr_shader_id: project::ShaderId,
    light_shader_id: project::ShaderId,
    main_shader_id: project::ShaderId,
    sky_shader_id: project::ShaderId,
) -> AppResult<ViewportId> {
    let dimension = Dimension::new("Main Dimension", size);
    let dimension_id = project.dimensions.register(dimension);

    let mut camera = Camera::new("Main Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_position((0.0, 5.0, 10.0));
    camera.set_pitch(cgmath::Deg(-20.0));
    camera.set_yaw(cgmath::Deg(-90.0));

    let camera_id = project.cameras.register(camera);

    let camera_uniform = Uniform::new(
        "Camera Buffer",
        vec![
            UniformField::new(
                "view_position",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Position),
            ),
            UniformField::new(
                "view",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::View),
            ),
            UniformField::new(
                "proj_view",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::ProjectionView,
                ),
            ),
            UniformField::new(
                "inv_proj",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::InverseProjection,
                ),
            ),
            UniformField::new(
                "inv_view",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::InverseView),
            ),
        ],
    );
    let camera_uniform_id = project.uniforms.register(camera_uniform);

    let camera_bind_group = BindGroup::new(
        "camera bind group",
        vec![BindGroupEntry::new(BindGroupResource::Uniform(Some(
            camera_uniform_id,
        )))],
    );
    let camera_bind_group_id = project.bind_groups.register(camera_bind_group);

    let light_uniform = Uniform::new(
        "light",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([2.0, 2.0, 2.0])),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 1.0, 1.0])),
            ),
        ],
    );
    let light_uniform_id = project.uniforms.register(light_uniform);

    let light_bind_group = BindGroup::new(
        "light bind group",
        vec![BindGroupEntry::new(BindGroupResource::Uniform(Some(
            light_uniform_id,
        )))],
    );
    let light_bind_group_id = project.bind_groups.register(light_bind_group);

    let image_texture_sampler_id = project.samplers.register(Sampler::new(
        "Image Texture Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..SamplerSpec::default()
        },
    ));

    let sky_sampler_id = project.samplers.register(Sampler::new(
        "Sky Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerSpec::default()
        },
    ));

    let mut cube_model = Model::load_from_obj_file("cube".to_string(), "cube.obj", device).await?;
    for material in cube_model.materials_mut() {
        let texture_paths = material.texture_paths();
        let diffuse_path = texture_paths.get(0).cloned().unwrap_or_default();
        let normal_path = texture_paths.get(1).cloned().unwrap_or_default();

        let diffuse_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let diffuse_texture = load_texture(&diffuse_path, diffuse_format).await?;
        let diffuse_id = project.textures.register(diffuse_texture);

        let normal_format = wgpu::TextureFormat::Rgba8Unorm;
        let normal_texture = load_texture(&normal_path, normal_format).await?;
        let normal_id = project.textures.register(normal_texture);

        let diffuse_view = TextureView::new(diffuse_path, Some(diffuse_id), None, None);
        let normal_view = TextureView::new(normal_path, Some(normal_id), None, None);

        let diffuse_texture_view_id = project.texture_views.register(diffuse_view);
        let normal_texture_view_id = project.texture_views.register(normal_view);

        let entries = vec![
            BindGroupEntry::new(BindGroupResource::Texture {
                texture_view_id: Some(diffuse_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new(BindGroupResource::Texture {
                texture_view_id: Some(normal_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new(BindGroupResource::Sampler {
                sampler_id: Some(image_texture_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ];

        let bind_group = BindGroup::new("cube bind group", entries);
        let bind_group_id = project.bind_groups.register(bind_group);

        material.set_bind_group_id(Some(bind_group_id));
    }

    let cube_model_id = project.models.register(cube_model);

    let hdr_texture = Texture::new(
        "Hdr Texture".to_string(),
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(dimension_id),
    );
    let hdr_texture_id = project.textures.register(hdr_texture);
    let hdr_texture_view = TextureView::new(
        "HDR Texture View".to_string(),
        Some(hdr_texture_id),
        None,
        None,
    );
    let hdr_texture_view_id = project.texture_views.register(hdr_texture_view);

    let hdr_viewport = project::viewport::Viewport::new(
        "HDR Buffer",
        Some(hdr_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    );
    let _ = project.viewports.register(hdr_viewport);

    let viewport_texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

    let hdr_bind_group = BindGroup::new(
        "HDR Bind Group",
        vec![
            BindGroupEntry::new(BindGroupResource::Texture {
                texture_view_id: Some(hdr_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new(BindGroupResource::Sampler {
                sampler_id: Some(image_texture_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    );

    let hdr_bind_group_id = project.bind_groups.register(hdr_bind_group);

    let hdr_loader = loader::HdrLoader::new(
        &device,
        project,
        runtime_project,
        recreate_tracker,
        equirectangular_shader_id,
    )?;
    let sky_bytes = resources::load_binary("pure-sky.hdr")
        .await
        .map_err(AppError::FileLoadError)?;
    let sky_texture_id = hdr_loader.from_equirectangular_bytes(
        project,
        runtime_project,
        recreate_tracker,
        &device,
        &queue,
        &sky_bytes,
        1080,
    )?;

    let sky_texture_view = TextureView::new(
        "Sky Texture View",
        Some(sky_texture_id),
        None,
        Some(wgpu::TextureViewDimension::Cube),
    );
    let sky_texture_view_id = project.texture_views.register(sky_texture_view);

    let environment_bind_group = BindGroup::new(
        "Environment Bind Group",
        vec![
            project::bindgroup::BindGroupEntry::new(
                project::bindgroup::BindGroupResource::Texture {
                    texture_view_id: Some(sky_texture_view_id),
                    view_dimension: wgpu::TextureViewDimension::Cube,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
            ),
            project::bindgroup::BindGroupEntry::new(
                project::bindgroup::BindGroupResource::Sampler {
                    sampler_id: Some(sky_sampler_id),
                    sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
                },
            ),
        ],
    );

    let environment_bind_group_id = project.bind_groups.register(environment_bind_group);

    let depth_texture = Texture::new(
        "depth texture",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(dimension_id),
    );
    let depth_texture_id = project.textures.register(depth_texture);
    let depth_texture_view =
        TextureView::new("Depth Texture View", Some(depth_texture_id), None, None);
    let depth_texture_view_id = project.texture_views.register(depth_texture_view);

    let viewport_texture = Texture::new(
        "Viewport Texture",
        viewport_texture_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(dimension_id),
    );
    let viewport_texture_id = project.textures.register(viewport_texture);
    let output_viewport_view_id = project.texture_views.register(TextureView::new(
        "Viewport Texture View".to_string(),
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_texture_view = TextureView::new(
        "Viewport Texture View Egui".to_string(),
        Some(viewport_texture_id),
        Some(TextureViewFormat::Linear),
        None,
    );
    let viewport_texture_view_id = project.texture_views.register(viewport_texture_view);
    let viewport = project::viewport::Viewport::new(
        "Viewport Texture",
        Some(viewport_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    );
    let viewport_id = project.viewports.register(viewport);

    let primitive_state = wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        unclipped_depth: false,
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
    };

    let mut main_render_pass = RenderPass::new(
        "Main Render Pass".to_string(),
        RenderPassTarget {
            texture_view_id: Some(hdr_texture_view_id),
            load_operation: LoadOperation::default(),
        },
        Some(RenderPassTarget {
            texture_view_id: Some(depth_texture_view_id),
            load_operation: LoadOperation::default(),
        }),
    );

    main_render_pass.add_pipeline(
        "light pipeline",
        primitive_state.clone(),
        Some(light_shader_id),
        Some(light_shader_id),
        vec![(0, camera_bind_group_id), (1, light_bind_group_id)],
        RenderDraw::Model {
            model_id: Some(cube_model_id),
            instances: 0..1,
            mesh_vertex_slot: 0,
            material_bind_group_slot: None,
        },
    );

    main_render_pass.add_pipeline(
        "models pipeline",
        primitive_state.clone(),
        Some(main_shader_id),
        Some(main_shader_id),
        vec![
            (1, camera_bind_group_id),
            (2, light_bind_group_id),
            (3, environment_bind_group_id),
        ],
        RenderDraw::Model {
            model_id: Some(cube_model_id),
            instances: 0..100,
            mesh_vertex_slot: 0,
            material_bind_group_slot: Some(0),
        },
    );

    main_render_pass.add_pipeline(
        "sky pipeline",
        primitive_state,
        Some(sky_shader_id),
        Some(sky_shader_id),
        vec![(0, camera_bind_group_id), (1, environment_bind_group_id)],
        RenderDraw::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
    );

    let main_render_pass_id = project.render_passes.register(main_render_pass);
    project.render_schedule.add(main_render_pass_id);

    let mut hdr_render_pass = RenderPass::new(
        "HDR render pass",
        RenderPassTarget {
            texture_view_id: Some(output_viewport_view_id),
            load_operation: LoadOperation::default(),
        },
        None,
    );

    hdr_render_pass.add_pipeline(
        "HDR pipeline",
        primitive_state,
        Some(hdr_shader_id),
        Some(hdr_shader_id),
        vec![(0, hdr_bind_group_id)],
        RenderDraw::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
    );

    let hdr_render_pass_id = project.render_passes.register(hdr_render_pass);
    project.render_schedule.add(hdr_render_pass_id);

    Ok(viewport_id)
}
