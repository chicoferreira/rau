use std::sync::Arc;

use egui_dnd::DragUpdate;
use slotmap::SecondaryMap;
use winit::{event::WindowEvent, window::Window};

use crate::{
    error::WgpuErrorScope,
    project::{
        self, BindGroupId, CameraId, DimensionId, ModelId, Project, ProjectResourceId,
        RenderPassId, RenderScheduleId, RuntimeProject, SamplerId, ShaderId, TextureId,
        TextureViewId, UniformId, ViewportId,
        bindgroup::{BindGroup, BindGroupCreationContext, BindGroupEntry, BindGroupResource},
        camera::{Camera, CameraCreationContext},
        dimension::Dimension,
        model::{MeshMaterialSelection, ModelCreationContext, vertex_buffer::VertexBufferField},
        render_pass,
        render_schedule::RenderScheduleContext,
        sampler::{Sampler, SamplerSpec},
        shader::{Shader, ShaderCreationContext},
        sync::{RuntimeCell, SyncResource, SyncTracker},
        texture::TextureCreationContext,
        texture_view::{TextureView, TextureViewCreationContext},
        uniform::{Uniform, UniformCreationContext, UniformField, UniformFieldSource},
        viewport::Viewport,
    },
    scene,
    ui::{
        self, Size2d,
        components::tiles::TreePane,
        panels::{inspector_pane::InspectorPane, viewport_pane::ViewportPane},
        rename::{RenameState, RenameTarget},
    },
    utils::{key::KeyboardState, resources},
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
    InspectResource(ProjectResourceId),
    OpenViewport(ViewportId),
    CreateRenderScheduleEntry,
    DeleteRenderScheduleEntry(usize),
    UpdateRenderScheduleEntry(usize, Option<RenderPassId>),
    ReorderRenderScheduleEntry(DragUpdate),
    CreateUniform,
    DeleteUniform(UniformId),
    CreateUniformField(UniformId, UniformFieldSource),
    UpdateUniformFieldSource(UniformId, usize, UniformFieldSource),
    DeleteUniformField(UniformId, usize),
    ReorderUniformField(UniformId, DragUpdate),
    StartRename(RenameTarget),
    CancelRename,
    ApplyRename(RenameTarget, String),
    CreateBindGroup,
    DeleteBindGroup(BindGroupId),
    CreateBindGroupEntry(BindGroupId, BindGroupResource),
    DeleteBindGroupEntry(BindGroupId, usize),
    UpdateBindGroupEntry(BindGroupId, usize, BindGroupEntry),
    ReorderBindGroupEntry(BindGroupId, DragUpdate),
    CreateViewport,
    DeleteViewport(ViewportId),
    CreateShader,
    DeleteShader(ShaderId),
    CreateCamera,
    DeleteCamera(CameraId),
    CreateDimension,
    DeleteDimension(DimensionId),
    CreateSampler,
    DeleteSampler(SamplerId),
    DeleteTexture(TextureId),
    CreateTextureView,
    DeleteTextureView(TextureViewId),
    AddModelVertexBufferField(ModelId, VertexBufferField),
    DeleteModelVertexBufferField(ModelId, usize),
    UpdateModelVertexBufferField(ModelId, usize, VertexBufferField),
    ReorderModelVertexBufferField(ModelId, DragUpdate),
    SetModelMaterialBindGroup(ModelId, usize, Option<BindGroupId>),
    SetMeshMaterialSelection(ModelId, usize, MeshMaterialSelection),
    CreateRenderPipeline(RenderPassId),
    DeleteRenderPipeline(RenderPassId, usize),
    ReorderRenderPipeline(RenderPassId, DragUpdate),
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
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
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
            anyhow::bail!("Surface capabilities does not include {EGUI_PREFERRED_SURFACE_FORMAT:?}")
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

        let mut egui_renderer = ui::renderer::EguiRenderer::new(&device, config.format, &window);

        let size = ui::Size2d::new(config.width, config.height);

        let mut project = project::Project::default();

        let equirectangular_shader = Shader::new(
            "Equirectengular Shader",
            resources::load_string("equirectangular.wgsl").await?,
        );
        let equirectengular_shader_id = project.shaders.register(equirectangular_shader);

        let hdr_shader = Shader::new("HDR Shader", resources::load_string("hdr.wgsl").await?);
        let hdr_shader_id = project.shaders.register(hdr_shader);

        let light_shader = Shader::new("Light Shader", resources::load_string("light.wgsl").await?);
        let light_shader_id = project.shaders.register(light_shader);

        let main_shader = Shader::new("Main Shader", resources::load_string("shader.wgsl").await?);
        let main_shader_id = project.shaders.register(main_shader);

        let sky_shader = Shader::new("Sky Shader", resources::load_string("sky.wgsl").await?);
        let sky_shader_id = project.shaders.register(sky_shader);

        let mut runtime_project = RuntimeProject::default();
        let mut recreate_tracker = SyncTracker::new();

        let viewport_id = scene::create_scene(
            &device,
            &queue,
            size,
            &mut project,
            &mut runtime_project,
            &mut recreate_tracker,
            &mut egui_renderer,
            equirectengular_shader_id,
            hdr_shader_id,
            light_shader_id,
            main_shader_id,
            sky_shader_id,
        )
        .await?;

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
            };

            snapshot.ui(
                ui,
                &mut self.inspector_tree_pane,
                &mut self.viewport_tree_pane,
            );
        });

        self.handle_events();
        self.tick_objects(dt, &mut encoder);

        self.egui_renderer.render_egui_frame(
            &frame,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
        );

        let submit_scope = WgpuErrorScope::push(&self.device);
        self.queue.submit(std::iter::once(encoder.finish()));
        if let Err(error) = submit_scope.pop() {
            self.runtime_project.render_schedule = RuntimeCell::Errored {
                at_revision: self.project.render_schedule.revision(),
                error,
            };
        }

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
        let mut tracker = SyncTracker::new();

        tracker.sync_storage(
            &mut self.project.dimensions,
            &mut self.runtime_project.dimensions,
            &mut (),
            &self.device,
        );

        let view = &mut TextureCreationContext {
            dimensions: &self.project.dimensions,
            device: &self.device,
            queue: &self.queue,
        };
        tracker.sync_storage(
            &mut self.project.textures,
            &mut self.runtime_project.textures,
            view,
            &self.device,
        );

        let view = &mut TextureViewCreationContext {
            textures: &self.project.textures,
            egui_renderer: &mut self.egui_renderer,
            device: &self.device,
            textures_runtime: &mut self.runtime_project.textures,
        };
        tracker.sync_storage(
            &mut self.project.texture_views,
            &mut self.runtime_project.texture_views,
            view,
            &self.device,
        );

        let view = &mut CameraCreationContext {
            dimensions: &self.project.dimensions,
            dt,
        };
        tracker.sync_storage(
            &mut self.project.cameras,
            &mut self.runtime_project.cameras,
            view,
            &self.device,
        );

        tracker.sync_storage(
            &mut self.project.samplers,
            &mut self.runtime_project.samplers,
            &mut &self.device,
            &self.device,
        );

        let view = &mut UniformCreationContext {
            cameras: &self.project.cameras,
            device: &self.device,
            queue: &self.queue,
        };
        tracker.sync_storage(
            &mut self.project.uniforms,
            &mut self.runtime_project.uniforms,
            view,
            &self.device,
        );

        let view = &mut BindGroupCreationContext {
            device: &self.device,
            runtime_uniforms: &mut self.runtime_project.uniforms,
            runtime_texture_views: &mut self.runtime_project.texture_views,
            runtime_samplers: &mut self.runtime_project.samplers,
        };
        tracker.sync_storage(
            &mut self.project.bind_groups,
            &mut self.runtime_project.bind_groups,
            view,
            &self.device,
        );

        let view = &mut ModelCreationContext {
            device: &self.device,
            queue: &self.queue,
        };
        tracker.sync_storage(
            &mut self.project.models,
            &mut self.runtime_project.models,
            view,
            &self.device,
        );

        let view = &mut ShaderCreationContext {
            device: &self.device,
        };
        tracker.sync_storage(
            &mut self.project.shaders,
            &mut self.runtime_project.shaders,
            view,
            &self.device,
        );

        let view = &mut render_pass::Context {
            device: &self.device,
            models: &self.project.models,
            runtime_shaders: &mut self.runtime_project.shaders,
            runtime_texture_views: &mut self.runtime_project.texture_views,
            runtime_bind_groups: &mut self.runtime_project.bind_groups,
        };
        tracker.sync_storage(
            &mut self.project.render_passes,
            &mut self.runtime_project.render_passes,
            view,
            &self.device,
        );

        let mut render_schedule_ctx = RenderScheduleContext {
            device: &self.device,
            encoder,
            render_passes: &self.project.render_passes,
            runtime_render_passes: &self.runtime_project.render_passes,
            models: &self.project.models,
            runtime_shaders: &self.runtime_project.shaders,
            runtime_texture_views: &self.runtime_project.texture_views,
            runtime_bind_groups: &self.runtime_project.bind_groups,
        };
        let _ = tracker.sync_singleton(
            RenderScheduleId,
            &mut self.project.render_schedule,
            &mut self.runtime_project.render_schedule,
            &mut render_schedule_ctx,
            &self.device,
        );
    }

    fn handle_events(&mut self) {
        for event in self.pending_events.drain(..) {
            log::debug!("Handling event {event:?}");
            match event {
                StateEvent::InspectResource(resource_id) => {
                    let pane = match resource_id {
                        ProjectResourceId::Uniform(id) => InspectorPane::Uniform(id),
                        ProjectResourceId::BindGroup(id) => InspectorPane::BindGroup(id),
                        ProjectResourceId::Shader(id) => InspectorPane::Shader(id),
                        ProjectResourceId::Camera(id) => InspectorPane::Camera(id),
                        ProjectResourceId::Dimension(id) => InspectorPane::Dimension(id),
                        ProjectResourceId::Sampler(id) => InspectorPane::Sampler(id),
                        ProjectResourceId::TextureView(id) => InspectorPane::TextureView(id),
                        ProjectResourceId::Viewport(id) => InspectorPane::Viewport(id),
                        ProjectResourceId::Texture(id) => InspectorPane::Texture(id),
                        ProjectResourceId::Model(id) => InspectorPane::Model(id),
                        ProjectResourceId::RenderPass(id) => InspectorPane::RenderPass(id),
                        ProjectResourceId::RenderSchedule(id) => InspectorPane::RenderSchedule(id),
                    };

                    self.inspector_tree_pane.add_pane(pane);
                }
                StateEvent::CreateRenderScheduleEntry => {
                    self.project.render_schedule.add(None);
                }
                StateEvent::DeleteRenderScheduleEntry(index) => {
                    self.project.render_schedule.remove_entry(index);
                }
                StateEvent::UpdateRenderScheduleEntry(index, render_pass_id) => {
                    self.project
                        .render_schedule
                        .update_entry(index, render_pass_id);
                }
                StateEvent::ReorderRenderScheduleEntry(drag_update) => {
                    self.project
                        .render_schedule
                        .reorder_entries(drag_update.from, drag_update.to);
                }
                StateEvent::OpenViewport(viewport_id) => {
                    self.viewport_tree_pane
                        .add_pane(ViewportPane { viewport_id });
                }
                StateEvent::CreateUniform => {
                    const DEFAULT_NAME: &str = "Uniform";

                    let uniform = Uniform::new(DEFAULT_NAME, vec![]);

                    let uniform_id = self.project.uniforms.register(uniform);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Uniform(uniform_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::StartRename(rename_target) => {
                    if let Some(current_name) = rename_target.get_label(&self.project) {
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label: current_name.to_string(),
                        });
                    }
                }
                StateEvent::CancelRename => {
                    self.rename_state = None;
                }
                StateEvent::ApplyRename(rename_target, new_name) => {
                    self.rename_state = None;
                    rename_target.apply(new_name, &mut self.project);
                }
                StateEvent::CreateUniformField(id, source) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(id) {
                        const DEFAULT_NAME: &str = "Field";

                        let index = uniform.fields().len();

                        uniform.add_field(UniformField::new(DEFAULT_NAME, source));

                        self.rename_state = Some(RenameState {
                            target: RenameTarget::UniformField(id, index),
                            current_label: DEFAULT_NAME.to_string(),
                        });
                    }
                }
                StateEvent::UpdateUniformFieldSource(uniform_id, index, source) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(uniform_id) {
                        uniform.set_field_source(index, source);
                    }
                }
                StateEvent::ReorderUniformField(uniform_id, drag_update) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(uniform_id) {
                        uniform.reorder_field(drag_update.from, drag_update.to);
                    }
                }
                StateEvent::CreateBindGroup => {
                    const DEFAULT_NAME: &str = "Bind Group";

                    let bind_group = BindGroup::new(DEFAULT_NAME, vec![]);
                    let bind_group_id = self.project.bind_groups.register(bind_group);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::BindGroup(bind_group_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateBindGroupEntry(id, resource) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.add_entry(BindGroupEntry::new_vertex_fragment(resource));
                    }
                }
                StateEvent::UpdateBindGroupEntry(id, index, entry) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.update_entry(index, entry);
                    }
                }
                StateEvent::ReorderBindGroupEntry(bind_group_id, drag_update) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(bind_group_id) {
                        bind_group.reorder_entries(drag_update.from, drag_update.to);
                    }
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
                StateEvent::CreateCamera => {
                    const DEFAULT_NAME: &str = "Camera";

                    let camera = Camera::new(DEFAULT_NAME.to_string());
                    let camera_id = self.project.cameras.register(camera);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Camera(camera_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateDimension => {
                    const DEFAULT_NAME: &str = "Dimension";

                    let dimension = Dimension::new(DEFAULT_NAME, Size2d::new(1920, 1080));
                    let dimension_id = self.project.dimensions.register(dimension);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Dimension(dimension_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateSampler => {
                    const DEFAULT_NAME: &str = "Sampler";

                    let sampler = Sampler::new(DEFAULT_NAME, SamplerSpec::default());
                    let sampler_id = self.project.samplers.register(sampler);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Sampler(sampler_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateViewport => {
                    const DEFAULT_NAME: &str = "Viewport";

                    let viewport = Viewport::new(DEFAULT_NAME, None, None, None);
                    let viewport_id = self.project.viewports.register(viewport);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Viewport(viewport_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateShader => {
                    const DEFAULT_NAME: &str = "Shader";
                    const DEFAULT_SOURCE: &str = r#"@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
"#;

                    let shader = Shader::new(DEFAULT_NAME, DEFAULT_SOURCE);
                    let shader_id = self.project.shaders.register(shader);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Shader(shader_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::CreateTextureView => {
                    const DEFAULT_NAME: &str = "Texture View";

                    let texture_view = TextureView::new(DEFAULT_NAME.to_string(), None, None, None);
                    let texture_view_id = self.project.texture_views.register(texture_view);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::TextureView(texture_view_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteUniform(id) => {
                    self.project.uniforms.unregister(id);
                }
                StateEvent::DeleteUniformField(id, index) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(id) {
                        uniform.remove_field(index);
                    }
                }
                StateEvent::DeleteBindGroup(bind_group_id) => {
                    for (_, model) in self.project.models.list_mut() {
                        for material in model.materials_mut() {
                            if material.bind_group_id() == Some(bind_group_id) {
                                material.set_bind_group_id(None);
                            }
                        }
                    }
                    self.project.bind_groups.unregister(bind_group_id);
                }
                StateEvent::DeleteBindGroupEntry(id, index) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.remove_entry(index);
                    }
                }
                StateEvent::DeleteViewport(viewport_id) => {
                    self.project.viewports.unregister(viewport_id);
                }
                StateEvent::DeleteShader(shader_id) => {
                    self.project.shaders.unregister(shader_id);
                }
                StateEvent::DeleteCamera(camera_id) => {
                    self.project.cameras.unregister(camera_id);
                }
                StateEvent::DeleteDimension(dimension_id) => {
                    self.project.dimensions.unregister(dimension_id);
                }
                StateEvent::DeleteSampler(sampler_id) => {
                    self.project.samplers.unregister(sampler_id);
                }
                StateEvent::DeleteTexture(texture_id) => {
                    self.project.textures.unregister(texture_id);
                }
                StateEvent::DeleteTextureView(texture_view_id) => {
                    self.project.texture_views.unregister(texture_view_id);
                }
                StateEvent::AddModelVertexBufferField(model_id, field) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        model.add_vertex_buffer_field(field);
                    }
                }
                StateEvent::DeleteModelVertexBufferField(model_id, index) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        model.remove_vertex_buffer_field(index);
                    }
                }
                StateEvent::UpdateModelVertexBufferField(model_id, index, field) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        model.set_vertex_buffer_field(index, field);
                    }
                }
                StateEvent::ReorderModelVertexBufferField(model_id, drag_update) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        model.reorder_vertex_buffer_field(drag_update.from, drag_update.to);
                    }
                }
                StateEvent::SetModelMaterialBindGroup(model_id, material_index, bind_group_id) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        if let Some(material) = model.materials_mut().get_mut(material_index) {
                            material.set_bind_group_id(bind_group_id);
                        }
                    }
                }
                StateEvent::SetMeshMaterialSelection(model_id, mesh_index, selection) => {
                    if let Ok(model) = self.project.models.get_mut(model_id) {
                        model.set_mesh_material_selection(mesh_index, selection);
                    }
                }
                StateEvent::CreateRenderPipeline(render_pass_id) => {
                    if let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) {
                        const DEFAULT_NAME: &str = "Pipeline";

                        let index = render_pass.pipelines.len();
                        render_pass.add_empty_pipeline(DEFAULT_NAME);

                        self.rename_state = Some(RenameState {
                            target: RenameTarget::RenderPipeline(render_pass_id, index),
                            current_label: DEFAULT_NAME.to_string(),
                        });
                    }
                }
                StateEvent::DeleteRenderPipeline(render_pass_id, index) => {
                    if let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) {
                        render_pass.remove_pipeline(index);
                    }
                }
                StateEvent::ReorderRenderPipeline(render_pass_id, drag_update) => {
                    if let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) {
                        render_pass.reorder_pipelines(drag_update.from, drag_update.to);
                    }
                }
            }
        }
    }
}
