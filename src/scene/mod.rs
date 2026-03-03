use wgpu::util::DeviceExt;

use crate::{
    model::{self, Vertex},
    project::{
        self, Project,
        uniform::{
            CameraFieldSource, Uniform, UniformData, UniformField, UniformFieldData,
            UniformFieldSource,
        },
    },
    render, resources,
    scene::hdr::HdrPipeline,
    state, texture,
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
    depth_texture: texture::Texture,
    camera_bind_group_id: project::BindGroupId,
    light_uniform_id: project::UniformId,
    light_bind_group_id: project::BindGroupId,
    light_render_pipeline: wgpu::RenderPipeline,
    hdr: hdr::HdrPipeline,
    environment_bind_group: wgpu::BindGroup,
    sky_pipeline: wgpu::RenderPipeline,
    hdr_texture_id: project::TextureId,
    pub viewport_texture_id: project::TextureId,
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
        let camera_uniform_data = UniformData {
            fields: vec![
                UniformField::new_camera_sourced("view_position", CameraFieldSource::Position),
                UniformField::new_camera_sourced("view", CameraFieldSource::View),
                UniformField::new_camera_sourced("proj_view", CameraFieldSource::ProjectionView),
                UniformField::new_camera_sourced("inv_proj", CameraFieldSource::InverseProjection),
                UniformField::new_camera_sourced("inv_view", CameraFieldSource::InverseView),
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
            "camera bind group",
            vec![project::bindgroup::BindGroupEntry {
                binding: 0,
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
            "light bind group",
            vec![project::bindgroup::BindGroupEntry {
                binding: 0,
                resource: project::bindgroup::BindGroupResource::Uniform(light_uniform_id),
            }],
        );
        let light_bind_group_id = project.bind_groups.register(light_bind_group);

        let obj_model =
            resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();

        let hdr_texture = project::texture::TextureEntry::new(
            "HDR BUffer",
            device,
            size,
            HdrPipeline::RENDER_FORMAT,
            egui_renderer,
        );
        let hdr_texture_id = project.textures.register(hdr_texture);
        let hdr_texture = &project.textures.get(hdr_texture_id).unwrap().texture;

        let viewport_texture_format = wgpu::TextureFormat::Rgba8UnormSrgb;

        let hdr = hdr::HdrPipeline::new(
            device,
            hdr_texture,
            viewport_texture_format,
            &project,
            hdr_shader_id,
        )?;

        let hdr_loader = loader::HdrLoader::new(&device, &project, equirectangular_shader_id)?;
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
                    resource: wgpu::BindingResource::TextureView(&sky_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sky_texture.sampler),
                },
            ],
        });

        let camera_bind_group = project.bind_groups.get(camera_bind_group_id).unwrap();
        let light_bind_group = project.bind_groups.get(light_bind_group_id).unwrap();
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group.layout,
                    &light_bind_group.layout,
                    &environment_layout,
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
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group.layout, &light_bind_group.layout],
                immediate_size: 0,
            });
            let shader = project.shaders.get(light_shader_id).unwrap();
            state::create_render_pipeline(
                "light pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let sky_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group.layout, &environment_layout],
                immediate_size: 0,
            });
            let shader = project.shaders.get(sky_shader_id).unwrap();
            state::create_render_pipeline(
                "sky pipeline",
                &device,
                &layout,
                HdrPipeline::RENDER_FORMAT,
                Some(texture::Texture::DEPTH_FORMAT),
                &[],
                wgpu::PrimitiveTopology::TriangleList,
                shader.create_wgpu_shader_module(device)?,
            )
        };

        let viewport_texture = project::texture::TextureEntry::new(
            "Viewport Texture",
            device,
            size,
            viewport_texture_format,
            egui_renderer,
        );
        let viewport_texture_id = project.textures.register(viewport_texture);

        Ok(Scene {
            render_pipeline,
            obj_model,
            instance_buffer,
            instances,
            depth_texture,
            camera_bind_group_id,
            light_uniform_id,
            light_bind_group_id,
            light_render_pipeline,
            hdr,
            environment_bind_group,
            sky_pipeline,
            hdr_texture_id,
            viewport_texture_id,
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

    pub fn resize(
        &mut self,
        size: ui::Size2d,
        project: &mut Project,
        device: &wgpu::Device,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        if let Some(hdr_texture) = project.textures.get_mut(self.hdr_texture_id) {
            hdr_texture.resize(size, device, egui_renderer);
            self.hdr.update_texture(device, &hdr_texture.texture);
        }

        if let Some(viewport_texture) = project.textures.get_mut(self.viewport_texture_id) {
            viewport_texture.resize(size, device, egui_renderer);
        }

        self.depth_texture = texture::Texture::create_depth_texture(device, size, "Depth Buffer");
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, project: &project::Project) {
        let camera_bind_group = project.bind_groups.get(self.camera_bind_group_id).unwrap();
        let light_bind_group = project.bind_groups.get(self.light_bind_group_id).unwrap();

        let main_render_pass = render::RenderPassSpec {
            label: Some("Main Render Pass"),
            target_spec: render::RenderPassTargetSpec {
                texture_id: self.hdr_texture_id,
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
                        render::RenderBindGroupSpec::new_fixed(0, &camera_bind_group.group),
                        render::RenderBindGroupSpec::new_fixed(1, &light_bind_group.group),
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
                        render::RenderBindGroupSpec::new_fixed(1, &camera_bind_group.group),
                        render::RenderBindGroupSpec::new_fixed(2, &light_bind_group.group),
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
                        render::RenderBindGroupSpec::new_fixed(0, &camera_bind_group.group),
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
