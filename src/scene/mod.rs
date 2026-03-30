use crate::{
    error::{AppError, AppResult, WgpuErrorScope},
    model::{self, Vertex},
    project::{
        self, BindGroupId, TextureViewId, ViewportId,
        camera::Camera,
        dimension::Dimension,
        sampler::{Sampler, SamplerSpec},
        texture::{Texture, TextureCreationContext, TextureSource},
        texture_view::{TextureView, TextureViewCreationContext, TextureViewFormat},
        uniform::{
            Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
        },
    },
    render, resources,
    scene::hdr::HdrPipeline,
    state,
    ui::{self},
};

mod hdr;
mod loader;

pub struct Scene {
    render_pipeline: wgpu::RenderPipeline,
    obj_model: model::Model,
    depth_texture_view_id: TextureViewId,
    camera_bind_group_id: BindGroupId,
    light_bind_group_id: BindGroupId,
    light_render_pipeline: wgpu::RenderPipeline,
    hdr: hdr::HdrPipeline,
    environment_bind_group_id: BindGroupId,
    sky_pipeline: wgpu::RenderPipeline,
    pub output_viewport_id: ViewportId,
    hdr_texture_view_id: TextureViewId,
    output_viewport_view_id: TextureViewId,
}

impl Scene {
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: ui::Size2d,
        project: &mut project::Project,
        egui_renderer: &mut ui::renderer::EguiRenderer,
        equirectangular_shader_id: project::ShaderId,
        hdr_shader_id: project::ShaderId,
        light_shader_id: project::ShaderId,
        main_shader_id: project::ShaderId,
        sky_shader_id: project::ShaderId,
    ) -> AppResult<Scene> {
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // normal map
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let dimension = Dimension {
            label: "Main Dimension".to_string(),
            size,
        };
        let dimension_id = project.dimensions.register(dimension);

        let mut camera = Camera::new("Main Camera".to_string());
        camera.set_dimension_id(Some(dimension_id));
        camera.set_position((0.0, 5.0, 10.0));
        camera.set_pitch(cgmath::Deg(-20.0));
        camera.set_yaw(cgmath::Deg(-90.0));

        let camera_id = project.cameras.register(camera);

        let camera_uniform = Uniform::new(
            device,
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
                    UniformFieldSource::new_camera_sourced(
                        Some(camera_id),
                        CameraField::InverseView,
                    ),
                ),
            ],
        )?;
        let camera_uniform_id = project.uniforms.register(camera_uniform);

        let camera_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "camera bind group".to_string(),
            vec![project::bindgroup::BindGroupEntry::new(
                project::bindgroup::BindGroupResource::Uniform(Some(camera_uniform_id)),
            )],
        )?;
        let camera_bind_group_id = project.bind_groups.register(camera_bind_group);

        let light_uniform = Uniform::new(
            device,
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
        )?;
        let light_uniform_id = project.uniforms.register(light_uniform);

        let light_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "light bind group".to_string(),
            vec![project::bindgroup::BindGroupEntry::new(
                project::bindgroup::BindGroupResource::Uniform(Some(light_uniform_id)),
            )],
        )?;
        let light_bind_group_id = project.bind_groups.register(light_bind_group);

        let image_texture_sampler_id = project.samplers.register(Sampler::new(
            device,
            "Image Texture Sampler".to_string(),
            SamplerSpec {
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                ..SamplerSpec::default()
            },
        )?);

        let sky_sampler_id = project.samplers.register(Sampler::new(
            device,
            "Sky Sampler".to_string(),
            SamplerSpec {
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..SamplerSpec::default()
            },
        )?);

        let obj_model = resources::load_model(
            project,
            "cube.obj",
            egui_renderer,
            &device,
            &queue,
            image_texture_sampler_id,
        )
        .await
        .unwrap();

        let hdr_texture = Texture::new(
            &TextureCreationContext {
                dimensions: &project.dimensions,
                device,
                queue,
            },
            "Hdr Texture".to_string(),
            HdrPipeline::RENDER_FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            TextureSource::Dimension(dimension_id),
        )?;
        let hdr_texture_id = project.textures.register(hdr_texture);
        let hdr_texture_view = TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            "HDR Texture View".to_string(),
            Some(hdr_texture_id),
            None,
            None,
        )?;
        let hdr_texture_view_id = project.texture_views.register(hdr_texture_view);

        let hdr_viewport = project::viewport::Viewport::new(
            "HDR Buffer",
            Some(hdr_texture_view_id),
            Some(dimension_id),
            Some(camera_id),
        )?;
        let _ = project.viewports.register(hdr_viewport);

        let viewport_texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let hdr = hdr::HdrPipeline::new(
            device,
            project,
            hdr_texture_view_id,
            viewport_texture_format,
            image_texture_sampler_id,
            hdr_shader_id,
        )?;

        let hdr_loader = loader::HdrLoader::new(&device, &project, equirectangular_shader_id)?;
        let sky_bytes = resources::load_binary("pure-sky.hdr")
            .await
            .map_err(AppError::FileLoadError)?;
        let sky_texture_id =
            hdr_loader.from_equirectangular_bytes(project, &device, &queue, &sky_bytes, 1080)?;

        let sky_texture_view = TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            "Sky Texture View".to_string(),
            Some(sky_texture_id),
            None,
            Some(wgpu::TextureViewDimension::Cube),
        )?;
        let sky_texture_view_id = project.texture_views.register(sky_texture_view);

        let environment_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "Environment Bind Group".to_string(),
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
        )?;

        let environment_bind_group_id = project.bind_groups.register(environment_bind_group);

        let camera_bind_group = project.bind_groups.get(camera_bind_group_id).unwrap();
        let light_bind_group = project.bind_groups.get(light_bind_group_id).unwrap();
        let environment_bind_group = project.bind_groups.get(environment_bind_group_id).unwrap();
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group.inner_layout()),
                    Some(&light_bind_group.inner_layout()),
                    Some(&environment_bind_group.inner_layout()),
                ],
                immediate_size: 0,
            });

        let render_pipeline = {
            let shader = project.shaders.get(main_shader_id).unwrap();
            state::create_render_pipeline(
                "normal shader pipeline",
                &device,
                &render_pipeline_layout,
                HdrPipeline::RENDER_FORMAT,
                Some(wgpu::TextureFormat::Depth32Float),
                &[model::ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group.inner_layout()),
                    Some(&light_bind_group.inner_layout()),
                ],
                immediate_size: 0,
            });
            let shader = project.shaders.get(light_shader_id).unwrap();
            state::create_render_pipeline(
                "light pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(wgpu::TextureFormat::Depth32Float),
                &[model::ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let sky_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_bind_group.inner_layout()),
                    Some(&environment_bind_group.inner_layout()),
                ],
                immediate_size: 0,
            });
            let shader = project.shaders.get(sky_shader_id).unwrap();
            state::create_render_pipeline(
                "sky pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(wgpu::TextureFormat::Depth32Float),
                &[],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let depth_texture = Texture::new(
            &TextureCreationContext {
                dimensions: &project.dimensions,
                device,
                queue,
            },
            "depth texture".to_string(),
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            TextureSource::Dimension(dimension_id),
        )?;
        let depth_texture_id = project.textures.register(depth_texture);
        let depth_texture_view = TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            "Depth Texture View".to_string(),
            Some(depth_texture_id),
            None,
            None,
        )?;
        let depth_texture_view_id = project.texture_views.register(depth_texture_view);

        let viewport_texture = Texture::new(
            &TextureCreationContext {
                dimensions: &project.dimensions,
                device,
                queue,
            },
            "Viewport Texture".to_string(),
            viewport_texture_format,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            TextureSource::Dimension(dimension_id),
        )?;
        let viewport_texture_id = project.textures.register(viewport_texture);
        let output_viewport_view_id = project.texture_views.register(TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            "Viewport Texture View".to_string(),
            Some(viewport_texture_id),
            Some(TextureViewFormat::Srgb),
            None,
        )?);
        let viewport_texture_view = TextureView::new(
            TextureViewCreationContext {
                textures: &project.textures,
                egui_renderer,
                device,
            },
            "Viewport Texture View Egui".to_string(),
            Some(viewport_texture_id),
            Some(TextureViewFormat::Linear),
            None,
        )?;
        let viewport_texture_view_id = project.texture_views.register(viewport_texture_view);
        let viewport = project::viewport::Viewport::new(
            "Viewport Texture",
            Some(viewport_texture_view_id),
            Some(dimension_id),
            Some(camera_id),
        )?;
        let viewport_id = project.viewports.register(viewport);

        Ok(Scene {
            render_pipeline,
            obj_model,
            depth_texture_view_id,
            camera_bind_group_id,
            light_bind_group_id,
            light_render_pipeline,
            hdr,
            environment_bind_group_id,
            sky_pipeline,
            output_viewport_id: viewport_id,
            hdr_texture_view_id,
            output_viewport_view_id,
        })
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        project: &project::Project,
    ) -> AppResult<()> {
        let depth_texture_view = project
            .texture_views
            .get(self.depth_texture_view_id)?
            .inner()
            .as_ref()
            .unwrap(); // TODO: FIX ME

        let scope = WgpuErrorScope::push(device);

        let main_render_pass = render::RenderPassSpec {
            label: Some("Main Render Pass"),
            target_spec: render::RenderPassTargetSpec {
                texture_view_id: self.hdr_texture_view_id,
                load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            },
            depth_spec: Some(render::RenderPassDepthSpec {
                texture: depth_texture_view,
                load_operation: wgpu::LoadOp::Clear(1.0),
            }),
            pipelines: vec![
                render::RenderPipelineSpec {
                    pipeline: &self.light_render_pipeline,
                    bind_groups: vec![
                        render::RenderBindGroupSpec::new_fixed(0, self.camera_bind_group_id),
                        render::RenderBindGroupSpec::new_fixed(1, self.light_bind_group_id),
                    ],
                    vertex_buffers: vec![render::RenderVertexBufferSpec::new_model_mesh(0)],
                    draw: render::RenderDrawSpec::Model {
                        model: &self.obj_model,
                        instances: 0..1,
                    },
                },
                render::RenderPipelineSpec {
                    pipeline: &self.render_pipeline,
                    bind_groups: vec![
                        render::RenderBindGroupSpec::new_model_material(0),
                        render::RenderBindGroupSpec::new_fixed(1, self.camera_bind_group_id),
                        render::RenderBindGroupSpec::new_fixed(2, self.light_bind_group_id),
                        render::RenderBindGroupSpec::new_fixed(3, self.environment_bind_group_id),
                    ],
                    vertex_buffers: vec![render::RenderVertexBufferSpec::new_model_mesh(0)],
                    draw: render::RenderDrawSpec::Model {
                        model: &self.obj_model,
                        instances: 0..100 as u32,
                    },
                },
                render::RenderPipelineSpec {
                    pipeline: &self.sky_pipeline,
                    bind_groups: vec![
                        render::RenderBindGroupSpec::new_fixed(0, self.camera_bind_group_id),
                        render::RenderBindGroupSpec::new_fixed(1, self.environment_bind_group_id),
                    ],
                    vertex_buffers: vec![],
                    draw: render::RenderDrawSpec::Single {
                        vertices: 0..3,
                        instances: 0..1,
                    },
                },
            ],
        };

        let hdr_pass = render::RenderPassSpec {
            label: Some("HDR Pass"),
            target_spec: render::RenderPassTargetSpec {
                texture_view_id: self.output_viewport_view_id,
                load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            },
            depth_spec: None,
            pipelines: vec![render::RenderPipelineSpec {
                pipeline: self.hdr.pipeline(),
                vertex_buffers: vec![],
                bind_groups: vec![render::RenderBindGroupSpec::new_fixed(
                    0,
                    self.hdr.bind_group_id,
                )],
                draw: render::RenderDrawSpec::Single {
                    vertices: 0..3,
                    instances: 0..1,
                },
            }],
        };

        let result = render::RenderPassSpecSet {
            render_passes: vec![main_render_pass, hdr_pass],
        };

        result.submit(encoder, project)?;
        scope.pop()?;

        Ok(())
    }
}
