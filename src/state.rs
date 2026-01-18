use std::sync::Arc;

use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    project, resources, scene,
    ui::{self},
};

pub enum StateEvent {
    SceneEvent(scene::SceneEvent),
    AddViewport(project::texture::TextureId),
}

impl From<scene::SceneEvent> for StateEvent {
    fn from(event: scene::SceneEvent) -> Self {
        StateEvent::SceneEvent(event)
    }
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    last_render_time: instant::Instant,
    egui_renderer: ui::renderer::EguiRenderer,
    scene: scene::Scene,
    pending_events: Vec<StateEvent>,
    app_tree: ui::pane::AppTree,
    adapter_info: wgpu::AdapterInfo,
    project: project::Project,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // The instance is used to create surfaces and adapters
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // The window we draw to
        let surface = instance.create_surface(window.clone()).unwrap();

        // The handle to the GPU
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let adapter_info = adapter.get_info();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        log::info!("Available surface formats: {:?}", surface_caps.formats);

        // Assume sRGB color profile
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        log::info!("Selected surface format: {:?}", surface_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format.remove_srgb_suffix(),
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        let mut egui_renderer =
            ui::renderer::EguiRenderer::new(&device, config.format.add_srgb_suffix(), &window);

        let mut project = project::Project::default();

        let equirectengular_shader = project.register_shader(
            "Equirectengular Shader",
            resources::load_string("equirectangular.wgsl")
                .await
                .unwrap(),
        );

        let hdr_shader = project.register_shader(
            "HDR Shader",
            resources::load_string("hdr.wgsl").await.unwrap(),
        );

        let light_shader = project.register_shader(
            "Light Shader",
            resources::load_string("light.wgsl").await.unwrap(),
        );

        let main_shader = project.register_shader(
            "Main Shader",
            resources::load_string("shader.wgsl").await.unwrap(),
        );

        let sky_shader = project.register_shader(
            "Sky Shader",
            resources::load_string("sky.wgsl").await.unwrap(),
        );

        let size = ui::Size2d::new(config.width, config.height);

        let scene = scene::Scene::new(
            &device,
            &queue,
            size,
            surface_format,
            &mut project,
            &mut egui_renderer,
            equirectengular_shader,
            hdr_shader,
            light_shader,
            main_shader,
            sky_shader,
        )
        .await?;

        let app_tree = ui::pane::AppTree::new_default();

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            last_render_time: instant::Instant::now(),
            egui_renderer,
            scene,
            pending_events: vec![],
            app_tree,
            adapter_info,
            project,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn render(&mut self, dt: instant::Duration) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let frame = self
            .egui_renderer
            .handle(&self.window, &screen_descriptor, |context| {
                // FIXME: until this new version is released with proper documentation
                #[allow(deprecated)]
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(0))
                    .show(context, |ui| {
                        let mut behavior = ui::pane::Behavior {
                            pending_events: &mut self.pending_events,
                            adapter_info: &self.adapter_info,
                            project: &mut self.project,
                            queue: &self.queue,
                        };

                        self.app_tree.ui(&mut behavior, ui);
                    });
            });

        let events = std::iter::once(scene::SceneEvent::Frame { dt }.into())
            .chain(self.pending_events.drain(..));

        for event in events {
            match event {
                StateEvent::SceneEvent(scene_event) => {
                    self.scene.handle_event(
                        scene_event,
                        &self.device,
                        &self.queue,
                        &mut self.project,
                        &mut self.egui_renderer,
                    );
                }
                StateEvent::AddViewport(texture_id) => {
                    self.app_tree.add_viewport(Some(texture_id));
                }
            }
        }

        self.scene.render(&mut encoder, &self.project);

        self.egui_renderer.render_egui_frame(
            &frame,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
            wgpu::Color {
                r: 0.02,
                g: 0.02,
                b: 0.02,
                a: 1.0,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn handle_window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: winit::event::WindowEvent,
    ) {
        let egui_response = self.egui_renderer.handle_input(&self.window, &event);
        if egui_response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = instant::Instant::now();
                let dt = now - self.last_render_time;
                self.last_render_time = now;
                match self.render(dt) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = self.window.inner_size();
                        self.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state) {
                (KeyCode::Escape, ElementState::Pressed) => event_loop.exit(),
                (key_code, element_state) => {
                    self.pending_events.push(
                        scene::SceneEvent::Keyboard {
                            key_code,
                            element_state,
                        }
                        .into(),
                    );
                }
            },
            _ => {}
        }
    }
}

pub fn create_render_pipeline(
    label: &str,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    topology: wgpu::PrimitiveTopology,
    shader: wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format.add_srgb_suffix(),
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology,
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
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
