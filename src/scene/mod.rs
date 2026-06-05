use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, Deg, Pitch, Yaw},
            compute_pass::{ComputePass, WorkGroups},
            dimension::Dimension,
            model::{Model, ModelRuntime},
            render_pass::{LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            sampler::{Sampler, SamplerSpec},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{
                Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    ui::size::Size2d,
};

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub async fn create_and_save_scene(
    app_file_system: &crate::file::file_system::AppFileSystem,
    device: &wgpu::Device,
) -> AppResult<()> {
    use std::path::PathBuf;

    use crate::file::{
        absolute::AbsolutePathBuf,
        file_system::ProjectFileSystemTrait,
        identifier::{ProjectIdentifier, ProjectSource},
    };

    let project_id = ProjectIdentifier::new(
        "full-example",
        AbsolutePathBuf::new(PathBuf::from("projects/full-example"))?,
    );
    let source = ProjectSource::Persistent(project_id);
    let (file_system, file_watcher) = app_file_system.mount_project(source.clone()).await?;

    let file_storage = FileStorage::new(source, file_system.clone(), file_watcher);

    let project = create_scene(&device, Size2d::new(1080, 1080), &file_storage).await?;

    let bytes = project.serialize()?;
    file_system.write(&FilePath::project_json(), bytes);

    Ok(())
}

#[allow(dead_code)]
pub async fn create_scene(
    device: &wgpu::Device,
    size: Size2d,
    file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let equirectangular_shader = Shader::new(
        "Equirectengular Shader",
        FilePath::from_str("equirectangular.wgsl")?,
    );
    let equirectengular_shader_id = project.shaders.register(equirectangular_shader);

    let hdr_shader = Shader::new("HDR Shader", FilePath::from_str("hdr.wgsl")?);
    let hdr_shader_id = project.shaders.register(hdr_shader);

    let light_shader = Shader::new("Light Shader", FilePath::from_str("light.wgsl")?);
    let light_shader_id = project.shaders.register(light_shader);

    let main_shader = Shader::new("Main Shader", FilePath::from_str("shader.wgsl")?);
    let main_shader_id = project.shaders.register(main_shader);

    let sky_shader = Shader::new("Sky Shader", FilePath::from_str("sky.wgsl")?);
    let sky_shader_id = project.shaders.register(sky_shader);

    let dimension = Dimension::new("Main Dimension", size);
    let dimension_id = project.dimensions.register(dimension);

    let mut camera = Camera::new("Main Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_position(glam::Vec3::new(0.0, 5.0, 10.0));
    camera.set_pitch(Pitch::new(Deg(-20.0)));
    camera.set_yaw(Yaw::new(Deg(-90.0)));

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
            UniformField::new("time", UniformFieldSource::new_time()),
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

    let cube_source = FilePath::from_str("cube.obj")?;
    let mut cube_model = Model::new("cube", cube_source.clone());
    let (cube_model_runtime, _) = ModelRuntime::load_from_obj_file(
        cube_source,
        &file_storage,
        cube_model.vertex_buffer_spec().clone(),
        device.clone(),
    )
    .await?; // temporary so we can set the material selection
    let mut material_bind_group_ids = cube_model.material_bind_group_ids().to_vec();
    for (material_index, material) in cube_model_runtime.materials().iter().enumerate() {
        let texture_paths = material.texture_paths();
        let diffuse_path = texture_paths.get(0).cloned().unwrap_or_default();
        let normal_path = texture_paths.get(1).cloned().unwrap_or_default();

        let diffuse_file_path = FilePath::from_str(diffuse_path.clone())?;
        let normal_file_path = FilePath::from_str(normal_path.clone())?;

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

        if material_bind_group_ids.len() <= material_index {
            material_bind_group_ids.resize(material_index + 1, None);
        }
        material_bind_group_ids[material_index] = Some(bind_group_id);
    }
    cube_model.set_material_bind_group_ids(material_bind_group_ids);

    let cube_model_id = project.models.register(cube_model);

    let hdr_texture_format = wgpu::TextureFormat::Rgba16Float;
    let hdr_texture = Texture::new(
        "Hdr Texture".to_string(),
        hdr_texture_format,
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
        FilePath::from_str("pure-sky.hdr")?,
        wgpu::TextureFormat::Rgba32Float,
    )?;
    let sky_texture_id = project.textures.register(sky_texture);

    let sky_texture_view = TextureView::new("label", Some(sky_texture_id), None, None);
    let sky_texture_view_id = project.texture_views.register(sky_texture_view);

    let dst_size = 1080;

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
                texture_view_id: Some(sky_texture_view_id),
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
        vec![Some(bind_group_id)],
        Some(equirectengular_shader_id),
        WorkGroups::new(num_workgroups, num_workgroups, 6),
    );

    project.compute_passes.register(compute_pass);

    let sky_texture_view = TextureView::new(
        "Sky Texture View",
        Some(dst_texture_id),
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

    let depth_texture_format = wgpu::TextureFormat::Depth32Float;
    let depth_texture = Texture::new(
        "depth texture",
        depth_texture_format,
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
        RenderPassTarget::new(Some(hdr_texture_view_id), LoadOperation::default()),
        Some(RenderPassTarget::new(
            Some(depth_texture_view_id),
            LoadOperation::default(),
        )),
    );

    let light_pipeline = RenderPipeline::new(
        "light pipeline",
        primitive_state,
        Some(light_shader_id),
        Some(light_shader_id),
        RenderDrawStrategy::Model {
            model_id: Some(cube_model_id),
            instances: 0..1,
            mesh_vertex_slot: 0,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
        ],
        hdr_texture_format,
        Some(depth_texture_format),
    );
    let light_pipeline_id = project.render_pipelines.register(light_pipeline);

    let models_pipeline = RenderPipeline::new(
        "models pipeline",
        primitive_state,
        Some(main_shader_id),
        Some(main_shader_id),
        RenderDrawStrategy::Model {
            model_id: Some(cube_model_id),
            instances: 0..100,
            mesh_vertex_slot: 0,
        },
        vec![
            BindGroupTarget::ModelMaterial,
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
            BindGroupTarget::Static(environment_bind_group_id),
        ],
        hdr_texture_format,
        Some(depth_texture_format),
    );
    let models_pipeline_id = project.render_pipelines.register(models_pipeline);

    let sky_pipeline = RenderPipeline::new(
        "sky pipeline",
        primitive_state,
        Some(sky_shader_id),
        Some(sky_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(environment_bind_group_id),
        ],
        hdr_texture_format,
        Some(depth_texture_format),
    );
    let sky_pipeline_id = project.render_pipelines.register(sky_pipeline);
    main_render_pass.set_pipelines(vec![light_pipeline_id, models_pipeline_id, sky_pipeline_id]);

    let main_render_pass_id = project.render_passes.register(main_render_pass);

    let mut hdr_render_pass = RenderPass::new(
        "HDR render pass",
        RenderPassTarget::new(Some(output_viewport_view_id), LoadOperation::default()),
        None,
    );

    let hdr_pipeline = RenderPipeline::new(
        "HDR pipeline",
        primitive_state,
        Some(hdr_shader_id),
        Some(hdr_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(hdr_bind_group_id)],
        viewport_texture_format,
        None,
    );

    let hdr_pipeline_id = project.render_pipelines.register(hdr_pipeline);

    hdr_render_pass.set_pipelines(vec![hdr_pipeline_id]);

    let hdr_render_pass_id = project.render_passes.register(hdr_render_pass);

    let entries = vec![main_render_pass_id, hdr_render_pass_id];
    project.presentation.set_render_passes(entries);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}

pub fn create_texture(path: FilePath, format: wgpu::TextureFormat) -> AppResult<Texture> {
    let label = path.to_string();
    let source = TextureSource::Image(Some(path));

    Ok(Texture::new(
        label,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        source,
    ))
}
