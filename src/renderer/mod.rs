mod camera;
mod egui_renderer;
mod gui;
mod model;
mod shader;
mod texture;
mod uniform;

use crate::renderer::egui_renderer::EguiRenderer;
use crate::renderer::uniform::GpuUniformAcessor;
use crate::{file, project};
use anyhow::Context;
use std::collections::HashMap;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::CursorGrabMode;

pub struct Renderer {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_texture: texture::DepthTexture,
    renderer_project: RendererProject,
    egui: EguiRenderer,
    last_render_time: instant::Instant,
    start_time: instant::Instant,
    mouse_pressed: bool,
}

pub struct RendererProject {
    project_render_pipeline: ProjectRenderPipeline,
    models: Vec<model::Model>,
    textures: Vec<texture::Texture>,
    // textures index -> egui texture id
    textures_egui: Vec<egui::TextureId>,
    viewport_clear_color: wgpu::Color,
    camera: camera::Camera,
    time_uniform: uniform::time::TimeUniform,
}

pub struct ProjectRenderPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_groups: Vec<(u32, BindGroupIdentifierType)>, // bind_group index, bind group type
}

pub enum BindGroupIdentifierType {
    Camera,
    Texture { texture_index: usize },
    Time,
}

impl Renderer {
    pub async fn new(
        window: winit::window::Window,
        project: &project::Project,
        window_size: PhysicalSize<u32>,
    ) -> anyhow::Result<Self> {
        let window = Arc::new(window);
        let (_instance, surface, _adapter, device, queue, config) =
            Self::init_wgpu(&window, window_size).await?;

        let camera_bind_group_layout = Self::create_camera_bind_group_layout(&device);
        let texture_bind_group_layout = Self::create_texture_bind_group_layout(&device);

        let camera = camera::Camera::from_project_camera(
            project.camera.clone(),
            config.width,
            config.height,
            &device,
            &camera_bind_group_layout,
            0,
        );

        let models = Self::load_models(&project.models, &device).await?;

        let textures = Self::load_textures(
            &project.textures,
            &device,
            &queue,
            &texture_bind_group_layout,
        )
        .await?;

        let shaders = Self::load_shaders(&project.shaders, &device).await?;

        let depth_texture =
            texture::DepthTexture::create_depth_texture(&device, &config, "Depth Texture");

        let time_uniform_layout =
            Self::create_vertex_fragment_bind_group_layout(&device, Some("time_uniform_layout"));

        let time_uniform =
            uniform::time::TimeUniform::new(&device, &time_uniform_layout, 0, Some("time_uniform"));

        let project_render_pipeline = Self::create_project_render_pipeline(
            project,
            &shaders,
            &device,
            config.format,
            &camera_bind_group_layout,
            &texture_bind_group_layout,
            &time_uniform_layout,
            &textures,
        )?;

        let mut egui = EguiRenderer::new(&device, config.format, None, 1, &window);

        let textures_egui = textures
            .iter()
            .map(|texture| egui.register_texture(&device, texture))
            .collect();

        Ok(Renderer {
            egui,
            window,
            surface,
            device,
            queue,
            config,
            depth_texture,
            renderer_project: RendererProject {
                project_render_pipeline,
                models,
                textures_egui,
                textures,
                viewport_clear_color: wgpu::Color {
                    r: project.viewport.clear_color[0],
                    g: project.viewport.clear_color[1],
                    b: project.viewport.clear_color[2],
                    a: project.viewport.clear_color[3],
                },
                camera,
                time_uniform,
            },
            last_render_time: instant::Instant::now(),
            start_time: instant::Instant::now(),
            mouse_pressed: false,
        })
    }

    async fn load_shaders(
        shaders: &[project::Shader],
        device: &wgpu::Device,
    ) -> anyhow::Result<HashMap<String, shader::Shader>> {
        let mut result = HashMap::new();
        for project_shader in shaders {
            let shader = shader::Shader::load(device, project_shader)
                .await
                .context("Failed to load shader")?;
            result.insert(project_shader.name.clone(), shader);
        }
        Ok(result)
    }

    async fn load_models(
        models: &[project::Model],
        device: &wgpu::Device,
    ) -> anyhow::Result<Vec<model::Model>> {
        let mut result = vec![];
        for model in models {
            let model = model::load_model_from_obj(&model.path, device).await?;
            result.push(model);
        }
        Ok(result)
    }

    fn create_texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
            ],
            label: Some("texture_bind_group_layout"),
        })
    }

    fn create_camera_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        Self::create_vertex_fragment_bind_group_layout(device, Some("camera_bind_group_layout"))
    }

    fn create_vertex_fragment_bind_group_layout(
        device: &wgpu::Device,
        label: Option<&str>,
    ) -> wgpu::BindGroupLayout {
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
            label,
        })
    }

    async fn init_wgpu(
        window: &Arc<winit::window::Window>,
        window_size: PhysicalSize<u32>,
    ) -> anyhow::Result<(
        wgpu::Instance,
        wgpu::Surface<'static>,
        wgpu::Adapter,
        wgpu::Device,
        wgpu::Queue,
        wgpu::SurfaceConfiguration,
    )> {
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .context("Failed to create surface")?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .context("Failed to request adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits {
                            max_texture_dimension_2d: 8192,
                            ..wgpu::Limits::downlevel_webgl2_defaults()
                        }
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("Failed to request device")?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .cloned()
            .unwrap_or(surface_capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok((instance, surface, adapter, device, queue, config))
    }

    async fn load_textures(
        textures: &[project::Texture],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> anyhow::Result<Vec<texture::Texture>> {
        let mut result = vec![];
        for texture in textures {
            let texture_bytes = file::load_file_bytes(&texture.path)
                .await
                .context("Failed to load texture")?;

            let label = texture
                .name
                .clone()
                .unwrap_or_else(|| texture.path.to_string_lossy().to_string());

            let texture = texture::Texture::from_bytes(
                device,
                queue,
                texture_bind_group_layout,
                &texture_bytes,
                label,
            )
            .context("Failed to load texture")?;

            result.push(texture);
        }
        Ok(result)
    }

    fn create_project_render_pipeline(
        project: &project::Project,
        shaders: &HashMap<String, shader::Shader>,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        time_bind_group_layout: &wgpu::BindGroupLayout,
        textures: &[texture::Texture],
    ) -> anyhow::Result<ProjectRenderPipeline> {
        let mut bind_groups = project.render_pipeline.bind_groups.clone();
        bind_groups.sort_by_key(|b| b.index);
        for (expected, bind_group) in bind_groups.iter().enumerate() {
            if expected != bind_group.index as usize {
                anyhow::bail!(
                    "Bind groups must be contiguous. Jump at index {}. Expected {}",
                    bind_group.index,
                    expected,
                );
            }
        }
        let bind_group_layouts: Vec<_> = bind_groups
            .iter()
            .map(|identifier| match &identifier.bind_group_type {
                project::BindGroupIdentifierType::Camera => camera_bind_group_layout,
                project::BindGroupIdentifierType::Texture { .. } => texture_bind_group_layout,
                &project::BindGroupIdentifierType::Time => time_bind_group_layout,
            })
            .collect();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bind_group_layouts,
                push_constant_ranges: &[],
            });

        let shader = shaders
            .get(&project.render_pipeline.shader.shader_name)
            .context(format!(
                "Failed to find shader: {}",
                project.render_pipeline.shader.shader_name
            ))?;
        let render_pipeline = create_render_pipeline(
            "Render Pipeline",
            device,
            &render_pipeline_layout,
            color_format,
            Some(texture::DepthTexture::DEPTH_FORMAT),
            &[model::Vertex::layout()],
            (shader.vertex(), shader.fragment()),
        );

        let bind_groups = bind_groups
            .into_iter()
            .map(|identifier| {
                let id_type = match identifier.bind_group_type {
                    project::BindGroupIdentifierType::Camera => BindGroupIdentifierType::Camera,
                    project::BindGroupIdentifierType::Texture { texture_name } => {
                        let texture_index = textures
                            .iter()
                            .position(|t| t.name == texture_name)
                            .with_context(|| format!("Failed to find texture: {}", texture_name))?;
                        BindGroupIdentifierType::Texture { texture_index }
                    }
                    project::BindGroupIdentifierType::Time => BindGroupIdentifierType::Time,
                };
                Ok((identifier.index, id_type))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(ProjectRenderPipeline {
            pipeline: render_pipeline,
            bind_groups,
        })
    }

    pub fn resize(&mut self, size: &PhysicalSize<u32>) {
        self.renderer_project.camera.resize(size.width, size.height);
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = texture::DepthTexture::create_depth_texture(
            &self.device,
            &self.config,
            "Depth Texture",
        );
    }

    pub fn window(&self) -> Arc<winit::window::Window> {
        self.window.clone()
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.renderer_project.viewport_clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            let pipeline = &self.renderer_project.project_render_pipeline;
            render_pass.set_pipeline(&pipeline.pipeline);

            for (index, bind_group_identifier) in &pipeline.bind_groups {
                let bind_group = match bind_group_identifier {
                    BindGroupIdentifierType::Camera => {
                        self.renderer_project.camera.get_bind_group()
                    }
                    BindGroupIdentifierType::Texture { texture_index, .. } => {
                        &self.renderer_project.textures[*texture_index].bind_group
                    }
                    &BindGroupIdentifierType::Time => self
                        .renderer_project
                        .time_uniform
                        .gpu_uniform
                        .get_bind_group(),
                };
                render_pass.set_bind_group(*index, bind_group, &[]);
            }

            for model in &self.renderer_project.models {
                for mesh in &model.meshes {
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass
                        .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..mesh.num_elements, 0, 0..1);
                }
            }
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window().scale_factor() as f32,
        };

        EguiRenderer::draw(self, &mut encoder, &view, screen_descriptor);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update_from_last_render_time(&mut self) {
        let now = instant::Instant::now();
        let dt = now - self.last_render_time;
        self.last_render_time = now;

        self.renderer_project.camera.update_camera(dt);
        self.renderer_project.camera.upload_gpu_uniform(&self.queue);

        self.renderer_project
            .time_uniform
            .update_time(&self.queue, self.start_time.elapsed().as_secs_f32());
    }

    pub fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        if self.egui.handle_input(&self.window, event) {
            return true;
        }

        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.renderer_project.camera.process_keyboard(*key, *state),
            WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    self.mouse_pressed = *state == winit::event::ElementState::Pressed;
                    if self.mouse_pressed {
                        self.window
                            .set_cursor_grab(CursorGrabMode::Confined)
                            .or_else(|_e| self.window.set_cursor_grab(CursorGrabMode::Locked))
                            .unwrap();

                        self.window.set_cursor_visible(false);
                    } else {
                        self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                        self.window.set_cursor_visible(true);
                    }
                    return true;
                }
                false
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
                true
            }
            WindowEvent::RedrawRequested => {
                self.window().request_redraw();
                self.update_from_last_render_time();

                match self.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it is lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        log::warn!("Surface lost or outdated, reconfiguring");
                        // self.resize(&self.window().inner_size())
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                    Err(wgpu::SurfaceError::Other) => log::error!("Other surface error"),
                }
                true
            }
            WindowEvent::Resized(size) => {
                self.resize(size);
                true
            }
            _ => false,
        }
    }

    pub fn handle_device_event(&mut self, event: &winit::event::DeviceEvent) -> bool {
        match event {
            winit::event::DeviceEvent::MouseMotion { delta } => {
                if self.mouse_pressed {
                    self.renderer_project.camera.process_mouse(delta.0, delta.1);
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}

fn create_render_pipeline(
    label: &str,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shaders: (&wgpu::ShaderModule, &wgpu::ShaderModule), // (vertex, fragment)
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shaders.0,
            entry_point: None,
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shaders.1,
            entry_point: None,
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}
