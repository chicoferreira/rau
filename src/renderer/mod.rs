mod camera;
mod egui_renderer;
mod gui;
mod shader;
mod uniform;
mod vertex;

use crate::project::Project;
use crate::renderer::egui_renderer::EguiRenderer;
use crate::renderer::uniform::GpuUniformAcessor;
use anyhow::Context;
use cgmath::{Deg, Point3, Rad};
use log::warn;
use std::sync::Arc;
use wgpu::util::DeviceExt;
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
    renderer_project: RendererProject,
    egui: EguiRenderer,
    last_render_time: instant::Instant,
    mouse_pressed: bool,
}

pub struct RendererProject {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    viewport_clear_color: wgpu::Color,
    camera: camera::Camera,
}

impl Renderer {
    pub async fn new(
        window: winit::window::Window,
        project: &Project,
        window_size: PhysicalSize<u32>,
    ) -> anyhow::Result<Self> {
        let window = Arc::new(window);
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

        let config = {
            let surface_capabilities = surface.get_capabilities(&adapter);
            // Shader code in this tutorial assumes an Srgb surface texture. Using a different
            // one will result all the colors comming out darker. If you want to support non
            // Srgb surfaces, you'll need to account for that when drawing to the frame.
            let surface_format = surface_capabilities
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .cloned()
                .unwrap_or(surface_capabilities.formats[0]);
            wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width,
                height,
                present_mode: wgpu::PresentMode::AutoNoVsync,
                alpha_mode: surface_capabilities.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            }
        };

        surface.configure(&device, &config);

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

        let camera = camera::Camera::new(
            Point3::new(0.0, 0.0, 5.0),
            Deg(-90.0),
            Rad(0.0),
            config.width,
            config.height,
            Deg(45.0),
            0.1,
            100.0,
            &device,
            &camera_bind_group_layout,
            0,
        );

        let render_pipeline = {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[&camera_bind_group_layout],
                    push_constant_ranges: &[],
                });

            let shader = shader::Shader::load(&device, &project.shader)
                .await
                .context("Failed to load shader")?;

            create_render_pipeline(
                "Render Pipeline",
                &device,
                &render_pipeline_layout,
                config.format,
                None,
                &[vertex::Vertex::desc()],
                (shader.vertex(), shader.fragment()),
            )
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertex::VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(vertex::INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = vertex::INDICES.len() as u32;

        let egui = EguiRenderer::new(&device, config.format, None, 1, &window);

        Ok(Renderer {
            egui,
            window,
            surface,
            device,
            queue,
            config,
            renderer_project: RendererProject {
                render_pipeline,
                vertex_buffer,
                index_buffer,
                num_indices,
                viewport_clear_color: wgpu::Color {
                    r: project.viewport.clear_color[0],
                    g: project.viewport.clear_color[1],
                    b: project.viewport.clear_color[2],
                    a: project.viewport.clear_color[3],
                },
                camera,
            },
            last_render_time: instant::Instant::now(),
            mouse_pressed: false,
        })
    }

    pub fn resize(&mut self, size: &PhysicalSize<u32>) {
        self.renderer_project.camera.resize(size.width, size.height);
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.device, &self.config);
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
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.renderer_project.render_pipeline);
            render_pass.set_vertex_buffer(0, self.renderer_project.vertex_buffer.slice(..));
            render_pass.set_index_buffer(
                self.renderer_project.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.set_bind_group(0, self.renderer_project.camera.get_bind_group(), &[]);
            render_pass.draw_indexed(0..self.renderer_project.num_indices, 0, 0..1);
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
                        warn!("Surface lost or outdated, reconfiguring");
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
