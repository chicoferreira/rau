use std::sync::Arc;

use wgpu::DownlevelFlags;
use winit::{event::WindowEvent, window::Window};

use crate::{
    error::AppResult,
    main_menu::MainMenu,
    ui::{self},
    utils::{event_queue::EventQueue, winit_runner::WindowApp},
    workspace::{AppContext, Workspace},
};

pub struct App {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    last_render_time: instant::Instant,
    egui_renderer: ui::renderer::EguiRenderer,
    backend: wgpu::Backend,
    downlevel_flags: wgpu::DownlevelFlags,
    limits: wgpu::Limits,
    state: State,
    event_queue: EventQueue<AppEvent>,
}

pub enum AppEvent {
    SetState(State),
}

pub enum State {
    MainMenu(MainMenu),
    Workspace(Workspace),
}

impl WindowApp for App {
    async fn new(window: Arc<winit::window::Window>) -> AppResult<Self> {
        let size = window.inner_size();

        let instance_descriptor = wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        };

        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(instance_descriptor);
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(instance_descriptor).await;

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let backend = adapter.get_info().backend;
        log::info!("Selected renderer backend: {backend:?}");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: adapter.limits(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let downlevel_capabilities = adapter.get_downlevel_capabilities();
        let downlevel_flags = downlevel_capabilities.flags;
        let surface_caps = surface.get_capabilities(&adapter);

        log::info!("Available surface formats: {:?}", surface_caps.formats);

        pub const SURFACE_FORMATS: &[wgpu::TextureFormat] = &[
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8Unorm,
        ];

        let surface_format = surface_caps
            .formats
            .into_iter()
            .find(|format| SURFACE_FORMATS.contains(format));

        let Some(surface_format) = surface_format else {
            panic!("Surface capabilities does not include any of {SURFACE_FORMATS:?}")
        };

        log::info!("Selected surface format: {:?}", surface_format);

        let supports_view_formats = downlevel_flags.contains(DownlevelFlags::SURFACE_VIEW_FORMATS);
        let surface_view_formats = if supports_view_formats {
            vec![surface_format.add_srgb_suffix()]
        } else {
            vec![]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: surface_view_formats,
            desired_maximum_frame_latency: 2,
        };

        let egui_renderer = ui::renderer::EguiRenderer::new(&device, config.format, &window);

        let state = State::MainMenu(MainMenu::default());

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            last_render_time: instant::Instant::now(),
            egui_renderer,
            backend,
            downlevel_flags,
            limits: adapter.limits(),
            state,
            event_queue: EventQueue::default(),
        })
    }

    fn handle_window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        event: WindowEvent,
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
                if let Err(e) = self.render(dt) {
                    log::error!("Render error: {e:?}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

impl App {
    fn handle_events(&mut self) {
        for event in self.event_queue.drain() {
            match event {
                AppEvent::SetState(state) => {
                    self.state = state;
                }
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn render(&mut self, dt: instant::Duration) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
            wgpu::CurrentSurfaceTexture::Lost => {
                // TODO: recreate devices
                anyhow::bail!("Lost device")
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        self.handle_events();

        let state = &mut self.state;
        let app_event_queue = &mut self.event_queue;
        let frame = self.egui_renderer.handle(&self.window, |ui| match state {
            State::MainMenu(main_menu) => main_menu.render_ui(ui),
            State::Workspace(workspace) => workspace.render_ui(ui, self.backend, app_event_queue),
        });

        match &mut self.state {
            State::MainMenu(main_menu) => main_menu.render(&mut self.event_queue),
            State::Workspace(workspace) => {
                let mut ctx = AppContext {
                    device: &self.device,
                    queue: &self.queue,
                    egui_renderer: &mut self.egui_renderer,
                    encoder: &mut encoder,
                    downlevel_flags: self.downlevel_flags,
                    limits: &self.limits,
                    dt,
                };
                workspace.render(&mut ctx);
            }
        }

        self.egui_renderer.render_egui_frame(
            &frame,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
        );

        // TODO: add validation for the frame_plan
        // let submit_scope = WgpuErrorScope::push(&self.device);
        self.queue.submit(std::iter::once(encoder.finish()));
        // if let Err(error) = submit_scope.pop() {
        //     self.runtime_project.frame_plan = RuntimeCell::Errored {
        //         at_revision: self.project.frame_plan.revision(),
        //         error,
        //     };
        // }

        output.present();

        Ok(())
    }
}
