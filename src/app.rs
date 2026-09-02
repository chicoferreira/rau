use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use crate::utils::benchmark::{Benchmark, BenchmarkSettings};
use crate::{
    StartupAction,
    error::AppResult,
    file::file_system::AppFileSystem,
    main_menu::MainMenu,
    ui::{self},
    utils::{
        event_queue::EventQueue,
        fps::FrameTimeTracker,
        render_settings::{self, RenderSettings},
        wgpu_error_scope::WgpuErrorScope,
    },
    workspace::{AppContext, Workspace},
};

#[derive(Default)]
pub struct AppSettings {
    pub action: StartupAction,
    /// Open a project's main viewport in the focus view, hiding the rest of the editor.
    pub focused: bool,
    #[cfg(not(target_arch = "wasm32"))]
    pub benchmark: BenchmarkSettings,
}

pub struct App {
    device: wgpu::Device,
    queue: wgpu::Queue,
    last_render_time: instant::Instant,
    egui_renderer: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    adapter_info: wgpu::AdapterInfo,
    downlevel_flags: wgpu::DownlevelFlags,
    app_file_system: AppFileSystem,
    state: State,
    event_queue: EventQueue<AppEvent>,
    frame_time: FrameTimeTracker,
    profiler: ui::profiler::Profiler,
    #[cfg(not(target_arch = "wasm32"))]
    benchmark: Option<Benchmark>,
}

pub enum AppEvent {
    SetState(State),
    SetPresentMode(wgpu::PresentMode),
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
}

pub enum State {
    MainMenu(MainMenu),
    Workspace(Workspace),
}

impl State {
    fn window_title(&self) -> String {
        match self {
            State::MainMenu(_) => "Rau".to_string(),
            State::Workspace(workspace) => format!("Rau - {}", workspace.project_name()),
        }
    }
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: AppSettings,
        app_file_system: AppFileSystem,
    ) -> AppResult<Self> {
        let render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("eframe was not set up with the wgpu renderer");

        setup_egui_context(&cc.egui_ctx);

        let adapter = &render_state.adapter;
        let adapter_info = adapter.get_info();
        log::info!("Selected renderer backend: {:?}", adapter_info.backend);
        log::info!("Selected adapter: {}", adapter_info.name);
        log::info!("Selected surface format: {:?}", render_state.target_format);

        let downlevel_flags = adapter.get_downlevel_capabilities().flags;

        let main_menu = MainMenu::with_startup_action(
            app_file_system.clone(),
            settings.action,
            settings.focused,
        );
        let state = State::MainMenu(main_menu);

        #[cfg(not(target_arch = "wasm32"))]
        let benchmark = Benchmark::new(settings.benchmark, &adapter_info);

        Ok(Self {
            device: render_state.device.clone(),
            queue: render_state.queue.clone(),
            last_render_time: instant::Instant::now(),
            egui_renderer: render_state.renderer.clone(),
            adapter_info,
            downlevel_flags,
            app_file_system,
            state,
            event_queue: EventQueue::default(),
            frame_time: FrameTimeTracker::new(),
            profiler: ui::profiler::Profiler::new(&render_state.device)?,
            #[cfg(not(target_arch = "wasm32"))]
            benchmark,
        })
    }

    fn handle_events(
        &mut self,
        #[cfg_attr(target_arch = "wasm32", allow(unused))] ctx: &egui::Context,
        frame: &mut eframe::Frame,
    ) {
        puffin::profile_function!();

        for event in self.event_queue.drain() {
            match event {
                AppEvent::SetState(state) => {
                    let title = state.window_title();
                    cfg_select! {
                        target_arch = "wasm32" => {
                            if let Err(e) = crate::utils::browser::set_document_title(&title) {
                                log::error!("Failed to set document title: {e:?}");
                            }
                        }
                        _ => ctx.send_viewport_cmd(egui::ViewportCommand::Title(title)),
                    }
                    self.state = state;
                }
                AppEvent::SetPresentMode(present_mode) => {
                    if let Some(surface_config) = frame.wgpu_surface_config() {
                        log::info!("Switching present mode to {present_mode:?}");
                        frame.set_wgpu_surface_config(egui_wgpu::SurfaceConfig {
                            present_mode,
                            ..surface_config
                        });
                    }
                }
                #[cfg(not(target_arch = "wasm32"))]
                AppEvent::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            }
        }
    }

    /// Ensures that puffin's scopes are only be enabled when
    /// the profiler is open, or when benchmarking is active.
    fn sync_profiler_scopes(&self) {
        let benchmarking = cfg_select! {
            target_arch = "wasm32" => false,
            _ => self.benchmark.is_some(),
        };

        let scopes_on = benchmarking || self.profiler.is_open();
        if scopes_on != puffin::are_scopes_on() {
            puffin::set_scopes_on(scopes_on);
        }
    }

    fn render(&mut self, dt: instant::Duration) {
        puffin::profile_function!();

        self.profiler.begin_frame();

        let submit_scope = match &mut self.state {
            State::MainMenu(main_menu) => {
                main_menu.render(&mut self.event_queue, &self.app_file_system);
                None
            }
            State::Workspace(workspace) => {
                let scope = WgpuErrorScope::push(&self.device);
                let mut ctx = AppContext {
                    device: &self.device,
                    queue: &self.queue,
                    egui_renderer: &self.egui_renderer,
                    downlevel_flags: self.downlevel_flags,
                    gpu_profiler: self.profiler.gpu_profiler(),
                    dt,
                };
                workspace.render(&mut ctx);
                Some(scope)
            }
        };

        if let (Some(scope), State::Workspace(workspace)) = (submit_scope, &mut self.state) {
            workspace.on_frame_submitted(scope.pop());
        }

        self.profiler.end_frame(&self.device, &self.queue);
    }
}

impl eframe::App for App {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {
        puffin::GlobalProfiler::lock().new_frame();
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        puffin::profile_scope!("frame");

        let now = instant::Instant::now();
        let dt = now - self.last_render_time;
        self.last_render_time = now;
        self.frame_time.update(dt);

        self.handle_events(ui.ctx(), frame);

        ui::fullscreen::handle_shortcut(ui.ctx());

        let present_mode = frame
            .wgpu_surface_config()
            .map_or(wgpu::PresentMode::AutoVsync, |config| config.present_mode);

        match &mut self.state {
            State::MainMenu(main_menu) => main_menu.render_ui(ui, &self.app_file_system),
            State::Workspace(workspace) => workspace.render_ui(
                ui,
                self.adapter_info.backend,
                present_mode,
                &self.frame_time,
                &mut self.profiler,
                &mut self.event_queue,
            ),
        }

        self.profiler.ui(ui.ctx());
        self.sync_profiler_scopes();

        self.render(dt);

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(benchmark) = &mut self.benchmark {
            benchmark.tick(dt, &self.state, present_mode, &mut self.event_queue);
        }

        ui.ctx().request_repaint();
    }
}

fn setup_egui_context(egui_context: &egui::Context) {
    ui::fonts::install(egui_context);
    ui::theme::install(egui_context);
    egui_extras::install_image_loaders(egui_context);

    // Disable the debug-build-only "widget rect changed id between passes" warning
    #[cfg(debug_assertions)]
    egui_context.all_styles_mut(|style| style.debug.warn_if_rect_changes_id = false);
}

pub fn wgpu_options(render_settings: &RenderSettings) -> egui_wgpu::WgpuConfiguration {
    egui_wgpu::WgpuConfiguration {
        surface: egui_wgpu::SurfaceConfig {
            present_mode: render_settings
                .present_mode
                .map_or(wgpu::PresentMode::AutoVsync, wgpu::PresentMode::from),
            desired_maximum_frame_latency: Some(2),
        },
        wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(egui_wgpu::WgpuSetupCreateNew {
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: render_settings
                    .backend
                    .map_or(render_settings::DEFAULT_BACKENDS, wgpu::Backends::from),
                ..egui_wgpu::WgpuSetupCreateNew::without_display_handle().instance_descriptor
            },
            device_descriptor: Arc::new(|adapter| {
                const OPTIONAL_FEATURES: wgpu::Features = wgpu::Features::POLYGON_MODE_LINE
                    .union(wgpu::Features::POLYGON_MODE_POINT)
                    .union(wgpu::Features::FLOAT32_FILTERABLE)
                    .union(wgpu::Features::TIMESTAMP_QUERY);

                wgpu::DeviceDescriptor {
                    label: Some("rau device"),
                    required_features: adapter.features() & OPTIONAL_FEATURES,
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    required_limits: adapter.limits(),
                    memory_hints: Default::default(),
                    trace: wgpu::Trace::Off,
                }
            }),
            ..egui_wgpu::WgpuSetupCreateNew::without_display_handle()
        }),
        ..Default::default()
    }
}
