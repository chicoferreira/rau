use wgpu::util::DeviceExt;

use crate::{
    camera,
    model::{self, Vertex},
    project, render, resources,
    scene::hdr::HdrPipeline,
    state, texture, ui,
};
use cgmath::{InnerSpace, Matrix, Rotation3, SquareMatrix, Zero};

mod hdr;
mod loader;

pub enum SceneEvent {
    Resize {
        size: ui::Size2d,
    },
    Scroll {
        delta_y_px: f32,
    },
    Drag {
        dx_px: f32,
        dy_px: f32,
    },
    Keyboard {
        key_code: winit::keyboard::KeyCode,
        element_state: winit::event::ElementState,
    },
    Frame {
        dt: instant::Duration,
    },
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view: [[f32; 4]; 4],
    view_proj: [[f32; 4]; 4],
    inv_proj: [[f32; 4]; 4],
    inv_view: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0; 4],
            view: cgmath::Matrix4::identity().into(),
            view_proj: cgmath::Matrix4::identity().into(),
            inv_proj: cgmath::Matrix4::identity().into(),
            inv_view: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        let proj = projection.calc_matrix();
        let view = camera.calc_matrix();
        let view_proj = proj * view;
        self.view = view.into();
        self.view_proj = view_proj.into();
        self.inv_proj = proj.invert().unwrap().into();
        self.inv_view = view.transpose().into();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightUniform {
    position: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: [f32; 3],
    // Due to uniforms requiring 16 byte (4 float) spacing, we need to use a padding field here
    _padding2: u32,
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
    depth_texture: texture::Texture,
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_uniform: LightUniform,
    light_render_pipeline: wgpu::RenderPipeline,
    hdr: hdr::HdrPipeline,
    environment_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    hdr_texture_id: project::TextureId,
    viewport_texture_id: project::TextureId,
}

impl Scene {
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: ui::Size2d,
        target_texture_format: wgpu::TextureFormat,
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
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // normal map
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let depth_texture = texture::Texture::create_depth_texture(&device, size, "depth texture");

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(size.width(), size.height(), cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

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

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let obj_model =
            resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let hdr_texture_id = project.register_texture(
            "HDR Buffer",
            device,
            size,
            HdrPipeline::RENDER_FORMAT,
            egui_renderer,
        );

        let hdr_texture = project.get_texture(hdr_texture_id).unwrap().texture();

        let hdr = hdr::HdrPipeline::new(
            device,
            hdr_texture,
            target_texture_format,
            &project,
            hdr_shader_id,
        );

        let hdr_loader = loader::HdrLoader::new(&device, &project, equirectangular_shader_id);
        let sky_bytes = resources::load_binary("pure-sky.hdr").await?;
        let sky_texture = hdr_loader.from_equirectangular_bytes(
            &device,
            &queue,
            &sky_bytes,
            1080,
            Some("Sky Texture"),
        )?;

        let environment_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("environment_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::Cube,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });

        let environment_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment_bind_group"),
            layout: &environment_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&sky_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sky_texture.sampler()),
                },
            ],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                    &environment_layout,
                ],
                immediate_size: 0,
            });

        let render_pipeline = {
            let shader = project.get_shader(main_shader_id).unwrap();
            state::create_render_pipeline(
                "normal shader pipeline",
                &device,
                &render_pipeline_layout,
                HdrPipeline::RENDER_FORMAT,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device),
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                immediate_size: 0,
            });
            let shader = project.get_shader(light_shader_id).unwrap();
            state::create_render_pipeline(
                "light pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device),
            )
        };

        let sky_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &environment_layout],
                immediate_size: 0,
            });
            let shader = project.get_shader(sky_shader_id).unwrap();
            state::create_render_pipeline(
                "sky pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(texture::Texture::DEPTH_FORMAT),
                &[],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device),
            )
        };

        let viewport_texture_id = project.register_texture(
            "Result Texture",
            device,
            size,
            target_texture_format,
            egui_renderer,
        );

        Ok(Scene {
            render_pipeline,
            obj_model,
            instance_buffer,
            instances,
            depth_texture,
            camera,
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            light_buffer,
            light_bind_group,
            light_uniform,
            light_render_pipeline,
            hdr,
            environment_bind_group,
            sky_pipeline,
            hdr_texture_id,
            viewport_texture_id,
        })
    }

    fn update(&mut self, queue: &wgpu::Queue, dt: instant::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position = (cgmath::Quaternion::from_axis_angle(
            (0.0, 1.0, 0.0).into(),
            cgmath::Deg(60.0 * dt.as_secs_f32()),
        ) * old_position)
            .into();
        queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }

    pub fn handle_event(
        &mut self,
        event: SceneEvent,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        project: &mut project::Project,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        match event {
            SceneEvent::Drag { dx_px, dy_px } => {
                self.camera_controller.handle_mouse(dx_px, dy_px);
            }
            SceneEvent::Scroll { delta_y_px } => {
                self.camera_controller.handle_scroll_pixels(delta_y_px);
            }
            SceneEvent::Resize { size } => {
                if let Some(hdr_texture) = project.get_texture_mut(self.hdr_texture_id) {
                    hdr_texture.resize(size, device, egui_renderer);
                    self.hdr.update_texture(device, hdr_texture.texture());
                }

                if let Some(viewport_texture) = project.get_texture_mut(self.viewport_texture_id) {
                    viewport_texture.resize(size, device, egui_renderer);
                }

                self.depth_texture =
                    texture::Texture::create_depth_texture(device, size, "Depth Buffer");
                self.projection.resize(size);

                self.camera_uniform
                    .update_view_proj(&self.camera, &self.projection);
                queue.write_buffer(
                    &self.camera_buffer,
                    0,
                    bytemuck::cast_slice(&[self.camera_uniform]),
                );
            }
            SceneEvent::Frame { dt } => {
                self.update(queue, dt);
            }
            SceneEvent::Keyboard {
                key_code,
                element_state,
            } => {
                self.camera_controller
                    .process_keyboard(key_code, element_state);
            }
        }
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, project: &project::Project) {
        let main_render_pass = render::RenderPassSpec {
            label: Some("Main Render Pass"),
            target_spec: render::RenderPassTargetSpec {
                texture_id: self.hdr_texture_id,
                texture_format: render::RenderPassTargetTextureFormat::UseExisting,
                load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            },
            depth_spec: Some(render::RenderPassDepthSpec {
                texture: &self.depth_texture.view,
                load_operation: wgpu::LoadOp::Clear(1.0),
            }),
            pipelines: vec![
                render::RenderPipelineSpec {
                    pipeline: &self.light_render_pipeline,
                    bind_groups: vec![
                        render::RenderBindGroupSpec::new_fixed(0, &self.camera_bind_group),
                        render::RenderBindGroupSpec::new_fixed(1, &self.light_bind_group),
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
                        render::RenderBindGroupSpec::new_fixed(1, &self.camera_bind_group),
                        render::RenderBindGroupSpec::new_fixed(2, &self.light_bind_group),
                        render::RenderBindGroupSpec::new_fixed(3, &self.environment_bind_group),
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
                        render::RenderBindGroupSpec::new_fixed(0, &self.camera_bind_group),
                        render::RenderBindGroupSpec::new_fixed(1, &self.environment_bind_group),
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
                texture_id: self.viewport_texture_id,
                texture_format: render::RenderPassTargetTextureFormat::NewViewSrgb,
                load_operation: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            },
            depth_spec: None,
            pipelines: vec![render::RenderPipelineSpec {
                pipeline: self.hdr.pipeline(),
                vertex_buffers: vec![],
                bind_groups: vec![render::RenderBindGroupSpec::new_fixed(
                    0,
                    self.hdr.bind_group(),
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
