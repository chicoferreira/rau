use crate::{
    error::AppResult,
    project::{
        self, Project, ViewportId,
        file::{FileSystem, ProjectFilePath},
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::Camera,
            dimension::Dimension,
            model::{Model, ModelRuntime},
            render_pass::{LoadOperation, RenderDraw, RenderPass, RenderPassTarget},
            sampler::{Sampler, SamplerSpec},
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{
                Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    ui::{self},
};

mod loader;

pub async fn create_scene(
    device: &wgpu::Device,
    size: ui::Size2d,
    project: &mut Project,
    file_system: &FileSystem,
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
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_uniform_id)),
        )],
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
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
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

    let mut cube_model = Model::new("cube", ProjectFilePath::new("cube.obj"));
    let cube_model_runtime = ModelRuntime::load_from_obj_file(
        cube_model.source().clone(),
        file_system.clone(),
        cube_model.vertex_buffer_spec().clone(),
        device.clone(),
    )
    .await?; // temporary so we can set the material selection
    for (material_index, material) in cube_model_runtime.materials().iter().enumerate() {
        let texture_paths = material.texture_paths();
        let diffuse_path = texture_paths.get(0).cloned().unwrap_or_default();
        let normal_path = texture_paths.get(1).cloned().unwrap_or_default();

        let diffuse_file_path = ProjectFilePath::new(diffuse_path.clone());
        let normal_file_path = ProjectFilePath::new(normal_path.clone());

        let diffuse_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let diffuse_texture = create_texture(diffuse_file_path, diffuse_format)?;
        let diffuse_id = project.textures.register(diffuse_texture);

        let normal_format = wgpu::TextureFormat::Rgba8Unorm;
        let normal_texture = create_texture(normal_file_path, normal_format)?;
        let normal_id = project.textures.register(normal_texture);

        let diffuse_view = TextureView::new(diffuse_path, Some(diffuse_id), None, None);
        let normal_view = TextureView::new(normal_path, Some(normal_id), None, None);

        let diffuse_texture_view_id = project.texture_views.register(diffuse_view);
        let normal_texture_view_id = project.texture_views.register(normal_view);

        let entries = vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(diffuse_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(normal_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(image_texture_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ];

        let bind_group = BindGroup::new("cube bind group", entries);
        let bind_group_id = project.bind_groups.register(bind_group);

        cube_model.set_material_bind_group_id(material_index, Some(bind_group_id));
    }

    let cube_model_id = project.models.register(cube_model);

    let hdr_texture = Texture::new(
        "Hdr Texture".to_string(),
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    );
    let hdr_texture_id = project.textures.register(hdr_texture);
    let hdr_texture_view = TextureView::new(
        "HDR Texture View".to_string(),
        Some(hdr_texture_id),
        None,
        None,
    );
    let hdr_texture_view_id = project.texture_views.register(hdr_texture_view);

    let hdr_viewport = Viewport::new(
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
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(hdr_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(image_texture_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    );

    let hdr_bind_group_id = project.bind_groups.register(hdr_bind_group);

    let sky_texture = create_texture(
        ProjectFilePath::new("pure-sky.hdr"),
        wgpu::TextureFormat::Rgba32Float,
    )?;
    let sky_texture_id = project.textures.register(sky_texture);

    let sky_texture_view = TextureView::new("label", Some(sky_texture_id), None, None);
    let sky_texture_view_id = project.texture_views.register(sky_texture_view);

    let sky_texture_id = loader::from_equirectangular_bytes(
        project,
        equirectangular_shader_id,
        sky_texture_view_id,
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
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(sky_texture_view_id),
                view_dimension: wgpu::TextureViewDimension::Cube,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sky_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
            }),
        ],
    );

    let environment_bind_group_id = project.bind_groups.register(environment_bind_group);

    let depth_texture = Texture::new(
        "depth texture",
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    );
    let depth_texture_id = project.textures.register(depth_texture);
    let depth_texture_view =
        TextureView::new("Depth Texture View", Some(depth_texture_id), None, None);
    let depth_texture_view_id = project.texture_views.register(depth_texture_view);

    let viewport_texture = Texture::new(
        "Viewport Texture",
        viewport_texture_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
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
    let viewport = Viewport::new(
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
        vec![
            (0, Some(camera_bind_group_id)),
            (1, Some(light_bind_group_id)),
        ],
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
            (1, Some(camera_bind_group_id)),
            (2, Some(light_bind_group_id)),
            (3, Some(environment_bind_group_id)),
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
        vec![
            (0, Some(camera_bind_group_id)),
            (1, Some(environment_bind_group_id)),
        ],
        RenderDraw::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
    );

    let main_render_pass_id = project.render_passes.register(main_render_pass);
    project.frame_plan.add(Some(main_render_pass_id));

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
        vec![(0, Some(hdr_bind_group_id))],
        RenderDraw::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
    );

    let hdr_render_pass_id = project.render_passes.register(hdr_render_pass);
    project.frame_plan.add(Some(hdr_render_pass_id));

    Ok(viewport_id)
}

pub fn create_texture(path: ProjectFilePath, format: wgpu::TextureFormat) -> AppResult<Texture> {
    let label = path.to_string();
    let source = TextureSource::Image(path);

    Ok(Texture::new(
        label,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        source,
    ))
}
