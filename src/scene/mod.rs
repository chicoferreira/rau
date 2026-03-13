use wgpu::util::DeviceExt;

use crate::{
    model::{self, Vertex},
    project::{
        self, BindGroupId, TextureViewId, UniformId, ViewportId,
        camera::Camera,
        dimension::Dimension,
        sampler::{Sampler, SamplerSpec},
        texture::{Texture, TextureCreationContext, TextureSource},
        texture_view::{TextureView, TextureViewCreationContext, TextureViewFormat},
        uniform::{
            CameraField, Uniform, UniformData, UniformField, UniformFieldData, UniformFieldSource,
        },
        viewport::ViewportCreationContext,
    },
    render, resources,
    scene::hdr::HdrPipeline,
    state,
    ui::{self},
};
use cgmath::{InnerSpace, Rotation3, Vector3, Zero};

mod hdr;
mod loader;

fn light_to_uniform_data(position: [f32; 3], color: [f32; 3]) -> project::uniform::UniformData {
    project::uniform::UniformData {
        fields: vec![
            project::uniform::UniformField::new_user_defined_vec3f("position", position),
            project::uniform::UniformField::new_user_defined_rgb("color", color),
        ],
    }
}

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        let model =
            cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation);
        InstanceRaw {
            model: model.into(),
            normal: cgmath::Matrix3::from(self.rotation).into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
}

impl model::Vertex for InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 10;

pub struct Scene {
    render_pipeline: wgpu::RenderPipeline,
    obj_model: model::Model,
    instance_buffer: wgpu::Buffer,
    instances: Vec<Instance>,
    depth_texture_view_id: TextureViewId,
    camera_bind_group_id: BindGroupId,
    light_uniform_id: UniformId,
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
    ) -> anyhow::Result<Scene> {
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

        let dimension = Dimension { size };
        let dimension_id = project.dimensions.register(dimension);

        let mut camera = Camera::new("Main Camera".to_string(), project, dimension_id);
        camera.set_position((0.0, 5.0, 10.0));
        camera.set_pitch(cgmath::Deg(-20.0));
        camera.set_yaw(cgmath::Deg(-90.0));

        let camera_id = project.cameras.register(camera);

        let camera_uniform_data = UniformData {
            fields: vec![
                UniformField::new_camera_sourced(
                    "view_position",
                    Some(camera_id),
                    CameraField::Position,
                ),
                UniformField::new_camera_sourced("view", Some(camera_id), CameraField::View),
                UniformField::new_camera_sourced(
                    "proj_view",
                    Some(camera_id),
                    CameraField::ProjectionView,
                ),
                UniformField::new_camera_sourced(
                    "inv_proj",
                    Some(camera_id),
                    CameraField::InverseProjection,
                ),
                UniformField::new_camera_sourced(
                    "inv_view",
                    Some(camera_id),
                    CameraField::InverseView,
                ),
            ],
        };
        let camera_uniform = Uniform::new(device, "Camera Buffer", camera_uniform_data);
        let camera_uniform_id = project.uniforms.register(camera_uniform);

        const SPACE_BETWEEN: f32 = 3.0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "camera bind group".to_string(),
            vec![project::bindgroup::BindGroupEntry {
                resource: project::bindgroup::BindGroupResource::Uniform(camera_uniform_id),
            }],
        );
        let camera_bind_group_id = project.bind_groups.register(camera_bind_group);

        let light_data = light_to_uniform_data([2.0, 2.0, 2.0], [1.0, 1.0, 1.0]);
        let light_uniform = Uniform::new(device, "light", light_data);
        let light_uniform_id = project.uniforms.register(light_uniform);

        let light_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "light bind group".to_string(),
            vec![project::bindgroup::BindGroupEntry {
                resource: project::bindgroup::BindGroupResource::Uniform(light_uniform_id),
            }],
        );
        let light_bind_group_id = project.bind_groups.register(light_bind_group);

        // TODO: Change this to some kind of default (use the same as when creating from the interface)
        let image_texture_sampler_id = project.samplers.register(Sampler::new(
            device,
            "Image Texture Sampler".to_string(),
            SamplerSpec::default(),
        ));

        let obj_model = resources::load_model(
            project,
            "cube.obj",
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
        );
        let hdr_texture_id = project.textures.register(hdr_texture);
        let hdr_texture_view = TextureView::new(
            &TextureViewCreationContext {
                textures: &project.textures,
            },
            "HDR Texture View".to_string(),
            hdr_texture_id,
            None,
            None,
            None,
        );
        let hdr_texture_view_id = project.texture_views.register(hdr_texture_view);

        let hdr_viewport = project::viewport::Viewport::new(
            ViewportCreationContext {
                texture_views: &project.texture_views,
                egui_renderer,
                device,
            },
            "HDR Buffer",
            hdr_texture_view_id,
            dimension_id,
            camera_id,
        );
        let hdr_viewport_id = project.viewports.register(hdr_viewport);
        let hdr_texture_id = project
            .viewports
            .get(hdr_viewport_id)
            .unwrap()
            .texture_view_id;

        let viewport_texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let hdr = hdr::HdrPipeline::new(
            device,
            project,
            hdr_texture_id,
            viewport_texture_format,
            image_texture_sampler_id,
            hdr_shader_id,
        )?;

        let hdr_loader = loader::HdrLoader::new(&device, &project, equirectangular_shader_id)?;
        let sky_bytes = resources::load_binary("pure-sky.hdr").await?;
        let sky_texture_id =
            hdr_loader.from_equirectangular_bytes(project, &device, &queue, &sky_bytes, 1080)?;

        let sky_texture_view = TextureView::new(
            &TextureViewCreationContext {
                textures: &project.textures,
            },
            "Sky Texture View".to_string(),
            sky_texture_id,
            None,
            Some(wgpu::TextureViewDimension::Cube),
            Some(6),
        );
        let sky_texture_view_id = project.texture_views.register(sky_texture_view);

        let environment_bind_group = project::bindgroup::BindGroup::new(
            project,
            device,
            "Environment Bind Group".to_string(),
            vec![
                project::bindgroup::BindGroupEntry {
                    resource: project::bindgroup::BindGroupResource::Texture {
                        texture_view_id: sky_texture_view_id,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                },
                project::bindgroup::BindGroupEntry {
                    resource: project::bindgroup::BindGroupResource::Sampler {
                        sampler_id: image_texture_sampler_id,
                        sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
                    },
                },
            ],
        );

        let environment_bind_group_id = project.bind_groups.register(environment_bind_group);

        let camera_bind_group = project.bind_groups.get(camera_bind_group_id).unwrap();
        let light_bind_group = project.bind_groups.get(light_bind_group_id).unwrap();
        let environment_bind_group = project.bind_groups.get(environment_bind_group_id).unwrap();
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group.inner_layout(),
                    &light_bind_group.inner_layout(),
                    &environment_bind_group.inner_layout(),
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
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_bind_group.inner_layout(),
                    &light_bind_group.inner_layout(),
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
                    &camera_bind_group.inner_layout(),
                    &environment_bind_group.inner_layout(),
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
        );
        let depth_texture_id = project.textures.register(depth_texture);
        let depth_texture_view = TextureView::new(
            &TextureViewCreationContext {
                textures: &project.textures,
            },
            "Depth Texture View".to_string(),
            depth_texture_id,
            None,
            None,
            None,
        );
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
        );
        let viewport_texture_id = project.textures.register(viewport_texture);
        let output_viewport_view_id = project.texture_views.register(TextureView::new(
            &TextureViewCreationContext {
                textures: &project.textures,
            },
            "Viewport Texture View".to_string(),
            viewport_texture_id,
            Some(TextureViewFormat::Srgb),
            None,
            None,
        ));
        let viewport_texture_view = TextureView::new(
            &TextureViewCreationContext {
                textures: &project.textures,
            },
            "Viewport Texture View Egui".to_string(),
            viewport_texture_id,
            Some(TextureViewFormat::Linear),
            None,
            None,
        );
        let viewport_texture_view_id = project.texture_views.register(viewport_texture_view);
        let viewport = project::viewport::Viewport::new(
            ViewportCreationContext {
                texture_views: &project.texture_views,
                egui_renderer,
                device,
            },
            "Viewport Texture",
            viewport_texture_view_id,
            dimension_id,
            camera_id,
        );
        let viewport_id = project.viewports.register(viewport);

        Ok(Scene {
            render_pipeline,
            obj_model,
            instance_buffer,
            instances,
            depth_texture_view_id,
            camera_bind_group_id,
            light_uniform_id,
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

    pub fn update(&mut self, project: &mut project::Project, dt: instant::Duration) {
        let light_uniform = project.uniforms.get_mut(self.light_uniform_id).unwrap();

        // this is fine for now
        let position = match &mut light_uniform.data.fields[0].source {
            UniformFieldSource::UserDefined(UniformFieldData::Vec3f(position)) => position,
            _ => unreachable!("deal with this later"),
        };

        let position_vec: Vector3<_> = position.clone().into();

        let new_position = cgmath::Quaternion::from_axis_angle(
            (0.0, 1.0, 0.0).into(),
            cgmath::Deg(60.0 * dt.as_secs_f32()),
        ) * position_vec;

        *position = new_position.into();
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, project: &project::Project) {
        let depth_texture_view = project
            .texture_views
            .get(self.depth_texture_view_id)
            .expect("deal with this later");

        let main_render_pass = render::RenderPassSpec {
            label: Some("Main Render Pass"),
            target_spec: render::RenderPassTargetSpec {
                texture_view_id: self.hdr_texture_view_id,
                load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            },
            depth_spec: Some(render::RenderPassDepthSpec {
                texture: depth_texture_view.inner(),
                load_operation: wgpu::LoadOp::Clear(1.0),
            }),
            pipelines: vec![
                render::RenderPipelineSpec {
                    pipeline: &self.light_render_pipeline,
                    bind_groups: vec![
                        render::RenderBindGroupSpec::new_fixed(0, self.camera_bind_group_id),
                        render::RenderBindGroupSpec::new_fixed(1, self.light_bind_group_id),
                    ],
                    vertex_buffers: vec![
                        render::RenderVertexBufferSpec::new_model_mesh(0),
                        render::RenderVertexBufferSpec::new_fixed(1, &self.instance_buffer),
                    ],
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
                    vertex_buffers: vec![
                        render::RenderVertexBufferSpec::new_model_mesh(0),
                        render::RenderVertexBufferSpec::new_fixed(1, &self.instance_buffer),
                    ],
                    draw: render::RenderDrawSpec::Model {
                        model: &self.obj_model,
                        instances: 0..self.instances.len() as u32,
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

        result.submit(encoder, project);
    }
}
