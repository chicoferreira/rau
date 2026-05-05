use std::sync::Arc;

use slotmap::SecondaryMap;
use winit::{event::WindowEvent, window::Window};

use crate::{
    error::AppResult,
    file_storage::FileStorage,
    fs::identifier::ProjectIdentifier,
    project::{
        self, DimensionId, FramePlanId, Project, ResourceId, ResourceKind, RuntimeProject,
        ViewportId,
        paths::FilePath,
        resource::{
            bindgroup::BindGroupCreationContext, camera::CameraCreationContext, compute_pass,
            frame_plan::FramePlanContext, model::ModelCreationContext, render_pass,
            shader::ShaderCreationContext, texture::TextureCreationContext,
            texture_view::TextureViewCreationContext, uniform::UniformCreationContext,
        },
        sync::SyncTracker,
    },
    scene,
    ui::{
        self,
        components::tiles::TreePane,
        panels::{inspector_pane::InspectorPane, viewport_pane::ViewportPane},
        rename::{RenameState, RenameTarget},
    },
    utils::key::KeyboardState,
};

#[derive(Debug, Clone)]
pub enum ViewportEvent {
    Resize { size: ui::Size2d },
    Scroll { delta_y_px: f32 },
    Drag { mouse_dx: f32, mouse_dy: f32 },
    KeyboardKeys { keyboard_state: KeyboardState },
    Focus,
}

#[derive(Debug, Clone)]
pub enum StateEvent {
    ViewportEvent(ViewportId, ViewportEvent),
    InspectResource(ResourceId),
    OpenViewport(ViewportId),
    CreateResource(ResourceKind),
    StartRename(RenameTarget),
    CancelRename,
    ApplyRename(RenameTarget, String),
    DeleteResource(ResourceId),
    CreateFile(FilePath),
    CreateFolder(FilePath),
    DeleteFile(FilePath),
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
    rename_state: Option<ui::rename::RenameState>,
    pending_events: Vec<StateEvent>,
    inspector_tree_pane: TreePane<InspectorPane>,
    viewport_tree_pane: TreePane<ViewportPane>,
    dimension_owners: SecondaryMap<DimensionId, ViewportId>,
    project: Project,
    runtime_project: RuntimeProject,
    tracker: SyncTracker,
    file_storage: FileStorage,
}

impl State {
    pub async fn new(window: Arc<Window>) -> AppResult<Self> {
        let size = window.inner_size();

        // The instance is used to create surfaces and adapters
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
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

        let surface_caps = surface.get_capabilities(&adapter);

        log::info!("Available surface formats: {:?}", surface_caps.formats);

        pub const EGUI_PREFERRED_SURFACE_FORMAT: wgpu::TextureFormat =
            wgpu::TextureFormat::Bgra8Unorm;

        let surface_format = if surface_caps
            .formats
            .contains(&EGUI_PREFERRED_SURFACE_FORMAT)
        {
            EGUI_PREFERRED_SURFACE_FORMAT
        } else {
            panic!("Surface capabilities does not include {EGUI_PREFERRED_SURFACE_FORMAT:?}")
        };

        log::info!("Selected surface format: {:?}", surface_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        let egui_renderer = ui::renderer::EguiRenderer::new(&device, config.format, &window);

        let size = ui::Size2d::new(config.width, config.height);

        let mut project = project::Project::default();

        let runtime_project = RuntimeProject::default();

        #[cfg(not(target_arch = "wasm32"))]
        let project_identifier = {
            let path = crate::fs::absolute::AbsolutePathBuf::try_from("res")?;
            ProjectIdentifier::new("res", path)
        };
        #[cfg(target_arch = "wasm32")]
        let project_identifier = ProjectIdentifier::new("res");

        let file_storage = FileStorage::new(project_identifier).await?;

        let viewport_id = scene::create_scene(&device, size, &mut project, &file_storage).await?;

        let inspector_tree_pane = TreePane::new("inspector");
        let mut viewport_tree_pane = TreePane::new("viewport");

        viewport_tree_pane.add_pane(ViewportPane { viewport_id });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            last_render_time: instant::Instant::now(),
            egui_renderer,
            dimension_owners: Default::default(),
            rename_state: None,
            pending_events: vec![],
            inspector_tree_pane,
            viewport_tree_pane,
            project,
            runtime_project,
            tracker: SyncTracker::default(),
            file_storage,
        })
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

        let frame = self.egui_renderer.handle(&self.window, |ui| {
            let mut snapshot = ui::pane::StateSnapshot {
                pending_events: &mut self.pending_events,
                project: &mut self.project,
                runtime_project: &mut self.runtime_project,
                rename_state: &mut self.rename_state,
                file_storage: &mut self.file_storage,
            };

            snapshot.ui(
                ui,
                &mut self.inspector_tree_pane,
                &mut self.viewport_tree_pane,
            );
        });

        self.handle_events();

        self.file_storage.tick(&mut self.tracker);

        for (_, camera) in self.project.cameras.list_mut() {
            camera.update(dt);
        }

        self.tick_objects(dt, &mut encoder);

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
                if let Err(e) = self.render(dt) {
                    log::error!("Render error: {e:?}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn tick_objects(&mut self, dt: std::time::Duration, encoder: &mut wgpu::CommandEncoder) {
        self.tracker.sync_storage(
            &mut self.project.dimensions,
            &mut self.runtime_project.dimensions,
            &mut (),
        );

        let view = &mut TextureCreationContext {
            dimensions: &self.project.dimensions,
            device: &self.device,
            queue: &self.queue,
            file_storage: &self.file_storage,
        };
        self.tracker.sync_storage(
            &mut self.project.textures,
            &mut self.runtime_project.textures,
            view,
        );

        let view = &mut TextureViewCreationContext {
            textures: &self.project.textures,
            egui_renderer: &mut self.egui_renderer,
            device: &self.device,
            textures_runtime: &mut self.runtime_project.textures,
        };
        self.tracker.sync_storage(
            &mut self.project.texture_views,
            &mut self.runtime_project.texture_views,
            view,
        );

        let view = &mut CameraCreationContext {
            dimensions: &self.project.dimensions,
            dt,
        };
        self.tracker.sync_storage(
            &mut self.project.cameras,
            &mut self.runtime_project.cameras,
            view,
        );

        self.tracker.sync_storage(
            &mut self.project.samplers,
            &mut self.runtime_project.samplers,
            &mut &self.device,
        );

        let view = &mut UniformCreationContext {
            cameras: &self.project.cameras,
            device: &self.device,
            queue: &self.queue,
            cameras_runtime: &mut self.runtime_project.cameras,
        };
        self.tracker.sync_storage(
            &mut self.project.uniforms,
            &mut self.runtime_project.uniforms,
            view,
        );

        let view = &mut BindGroupCreationContext {
            device: &self.device,
            runtime_uniforms: &mut self.runtime_project.uniforms,
            runtime_texture_views: &mut self.runtime_project.texture_views,
            runtime_samplers: &mut self.runtime_project.samplers,
        };
        self.tracker.sync_storage(
            &mut self.project.bind_groups,
            &mut self.runtime_project.bind_groups,
            view,
        );

        let view = &mut ModelCreationContext {
            device: &self.device,
            queue: &self.queue,
            file_storage: &self.file_storage,
        };
        self.tracker.sync_storage(
            &mut self.project.models,
            &mut self.runtime_project.models,
            view,
        );

        let view = &mut ShaderCreationContext {
            device: &self.device,
            file_storage: &self.file_storage,
        };
        self.tracker.sync_storage(
            &mut self.project.shaders,
            &mut self.runtime_project.shaders,
            view,
        );

        let view = &mut render_pass::Context {
            device: &self.device,
            models: &self.project.models,
            runtime_models: &self.runtime_project.models,
            runtime_shaders: &mut self.runtime_project.shaders,
            runtime_texture_views: &mut self.runtime_project.texture_views,
            runtime_bind_groups: &mut self.runtime_project.bind_groups,
        };
        self.tracker.sync_storage(
            &mut self.project.render_passes,
            &mut self.runtime_project.render_passes,
            view,
        );

        let view = &mut compute_pass::Context {
            device: &self.device,
            encoder,
            runtime_shaders: &mut self.runtime_project.shaders,
            runtime_bind_groups: &mut self.runtime_project.bind_groups,
        };
        self.tracker.sync_storage(
            &mut self.project.compute_passes,
            &mut self.runtime_project.compute_passes,
            view,
        );

        let mut frame_plan_ctx = FramePlanContext {
            device: &self.device,
            encoder,
            render_passes: &self.project.render_passes,
            runtime_render_passes: &self.runtime_project.render_passes,
            models: &self.project.models,
            runtime_models: &self.runtime_project.models,
            runtime_shaders: &self.runtime_project.shaders,
            runtime_texture_views: &self.runtime_project.texture_views,
            runtime_bind_groups: &self.runtime_project.bind_groups,
        };
        let _ = self.tracker.sync_singleton(
            FramePlanId,
            &mut self.project.frame_plan,
            &mut self.runtime_project.frame_plan,
            &mut frame_plan_ctx,
        );

        self.tracker.clear_changes();
    }

    fn handle_events(&mut self) {
        for event in self.pending_events.drain(..) {
            log::debug!("Handling event {event:?}");
            match event {
                StateEvent::InspectResource(resource_id) => {
                    let pane = match resource_id {
                        ResourceId::Uniform(id) => InspectorPane::Uniform(id),
                        ResourceId::BindGroup(id) => InspectorPane::BindGroup(id),
                        ResourceId::Shader(id) => InspectorPane::Shader(id),
                        ResourceId::Camera(id) => InspectorPane::Camera(id),
                        ResourceId::Dimension(id) => InspectorPane::Dimension(id),
                        ResourceId::Sampler(id) => InspectorPane::Sampler(id),
                        ResourceId::TextureView(id) => InspectorPane::TextureView(id),
                        ResourceId::Viewport(id) => InspectorPane::Viewport(id),
                        ResourceId::Texture(id) => InspectorPane::Texture(id),
                        ResourceId::Model(id) => InspectorPane::Model(id),
                        ResourceId::RenderPass(id) => InspectorPane::RenderPass(id),
                        ResourceId::FramePlan(id) => InspectorPane::FramePlan(id),
                        ResourceId::ComputePass(id) => InspectorPane::ComputePass(id),
                    };

                    self.inspector_tree_pane.add_pane(pane);
                }
                StateEvent::CreateResource(kind) => {
                    let rename_target = RenameTarget::CreateResource(kind);
                    if let Some(label) = rename_target.get_rename_label(&self.project) {
                        let current_label = label.to_string();
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label,
                        });
                    }
                }
                StateEvent::DeleteResource(id) => {
                    self.project.unregister(id);
                    self.runtime_project.unregister(id);
                    self.tracker.push_resource_change(id);
                }
                StateEvent::CreateFile(parent_path) => {
                    let rename_target = RenameTarget::CreateFile(parent_path);
                    if let Some(label) = rename_target.get_rename_label(&self.project) {
                        let current_label = label.to_string();
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label,
                        });
                    }
                }
                StateEvent::CreateFolder(parent_path) => {
                    let rename_target = RenameTarget::CreateFolder(parent_path);
                    if let Some(label) = rename_target.get_rename_label(&self.project) {
                        let current_label = label.to_string();
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label,
                        });
                    }
                }
                StateEvent::DeleteFile(file_path) => {
                    self.file_storage.delete_file_in_background(file_path);
                }
                StateEvent::OpenViewport(viewport_id) => {
                    self.viewport_tree_pane
                        .add_pane(ViewportPane { viewport_id });
                }
                StateEvent::StartRename(rename_target) => {
                    if let Some(current_name) = rename_target.get_rename_label(&self.project) {
                        let current_label = current_name.to_string();
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label,
                        });
                    }
                }
                StateEvent::CancelRename => {
                    self.rename_state = None;
                }
                StateEvent::ApplyRename(rename_target, new_name) => {
                    self.rename_state = None;
                    rename_target.apply(new_name, &mut self.project, &mut self.file_storage);
                }
                StateEvent::ViewportEvent(viewport_id, viewport_event) => {
                    if let Ok(viewport) = self.project.viewports.get_mut(viewport_id) {
                        match viewport_event {
                            ViewportEvent::Resize { size } => {
                                // set the requested_ui_size so:
                                // 1. the viewport doesn't keep sending resize events when it doesn't match the actual size of the viewport
                                // 2. we know to which size to resize the camera when the viewport gets focused (handled in the event below)
                                viewport.requested_ui_size = Some(size);

                                if let Some(dimension_id) = viewport.dimension_id {
                                    let is_owner = self
                                        .dimension_owners
                                        .get(dimension_id)
                                        .map_or(true, |&owner| owner == viewport_id);

                                    // relevant issue: https://github.com/chicoferreira/rau/issues/8
                                    //
                                    // this is a bit hacky, but we want the camera to resize immediately if this
                                    // viewport is the owner of the dimension or the viewport has no owners, otherwise
                                    // we'll wait until it gets focused (handled in the event below). this avoids the
                                    // problem of fighting when there are two viewports with different sizes for the same dimension.
                                    // this way, only one of them (the owner) will control the dimension size.
                                    if is_owner {
                                        if let Ok(dimension) =
                                            self.project.dimensions.get_mut(dimension_id)
                                        {
                                            dimension.set_size(size);
                                        }
                                    }
                                }
                            }
                            ViewportEvent::Focus => {
                                // read the comment in the event above for more context
                                if let Some(dimension_id) = viewport.dimension_id {
                                    self.dimension_owners.insert(dimension_id, viewport_id);
                                    if let Some(ui_size) = viewport.requested_ui_size {
                                        if let Ok(dimension) =
                                            self.project.dimensions.get_mut(dimension_id)
                                        {
                                            dimension.set_size(ui_size);
                                        }
                                    }
                                }
                            }
                            ViewportEvent::Scroll { delta_y_px } => {
                                if let Some(camera_id) = viewport.controls_camera_id
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.input_mut().handle_scroll_pixels(delta_y_px);
                                }
                            }
                            ViewportEvent::Drag { mouse_dx, mouse_dy } => {
                                if let Some(camera_id) = viewport.controls_camera_id
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.input_mut().handle_mouse(mouse_dx, mouse_dy);
                                }
                            }
                            ViewportEvent::KeyboardKeys { keyboard_state } => {
                                if let Some(camera_id) = viewport.controls_camera_id
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.input_mut().handle_keyboard(keyboard_state);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
