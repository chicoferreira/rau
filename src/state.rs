use std::sync::Arc;

use egui_dnd::DragUpdate;
use slotmap::SecondaryMap;
use winit::{event::WindowEvent, window::Window};

use crate::{
    error::{AppResult, SourcedError},
    key::KeyboardState,
    project::{
        self, BindGroupId, CameraId, DimensionId, ProjectResourceId, SamplerId, UniformId,
        ViewportId,
        bindgroup::{BindGroupCreationContext, BindGroupEntry, BindGroupResource},
        camera::{Camera, CameraCreationContext},
        dimension::Dimension,
        recreate::RecreateTracker,
        sampler::{Sampler, SamplerSpec},
        texture::TextureCreationContext,
        texture_view::TextureViewCreationContext,
        uniform::{UniformCreationContext, UniformField, UniformFieldSource},
        viewport::ViewportCreationContext,
    },
    resources, scene,
    ui::{
        self, Size2d,
        components::tiles::TreePane,
        panels::{inspector_pane::InspectorPane, viewport_pane::ViewportPane},
        rename::{RenameState, RenameTarget},
    },
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
    UpdateBindGroupEntry(BindGroupId, usize, BindGroupResource),
    ReorderBindGroupEntry(BindGroupId, DragUpdate),
    CreateCamera,
    DeleteCamera(CameraId),
    CreateDimension,
    DeleteDimension(DimensionId),
    CreateSampler,
    DeleteSampler(SamplerId),
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
    rename_state: Option<ui::rename::RenameState>,
    pending_events: Vec<StateEvent>,
    inspector_tree_pane: TreePane<InspectorPane>,
    viewport_tree_pane: TreePane<ViewportPane>,
    dimension_owners: SecondaryMap<DimensionId, ViewportId>,
    errors: Vec<SourcedError>,
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
            wgpu::TextureFormat::Rgba8Unorm;

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

        let mut project = project::Project::new();

        let equirectangular_shader = project::shader::Shader::new(
            "Equirectengular Shader",
            resources::load_string("equirectangular.wgsl")
                .await
                .unwrap(),
        );
        let equirectengular_shader_id = project.shaders.register(equirectangular_shader);

        let hdr_shader = project::shader::Shader::new(
            "HDR Shader",
            resources::load_string("hdr.wgsl").await.unwrap(),
        );
        let hdr_shader_id = project.shaders.register(hdr_shader);

        let light_shader = project::shader::Shader::new(
            "Light Shader",
            resources::load_string("light.wgsl").await.unwrap(),
        );
        let light_shader_id = project.shaders.register(light_shader);

        let main_shader = project::shader::Shader::new(
            "Main Shader",
            resources::load_string("shader.wgsl").await.unwrap(),
        );
        let main_shader_id = project.shaders.register(main_shader);

        let sky_shader = project::shader::Shader::new(
            "Sky Shader",
            resources::load_string("sky.wgsl").await.unwrap(),
        );
        let sky_shader_id = project.shaders.register(sky_shader);

        let scene = scene::Scene::new(
            &device,
            &queue,
            size,
            &mut project,
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

        viewport_tree_pane.add_pane(ViewportPane {
            viewport_id: scene.output_viewport_id,
        });

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
            scene,
            rename_state: None,
            pending_events: vec![],
            inspector_tree_pane,
            viewport_tree_pane,
            project,
            errors: vec![],
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

    pub fn render(&mut self, dt: instant::Duration) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

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

        let frame = self
            .egui_renderer
            .handle(&self.window, &screen_descriptor, |context| {
                // FIXME: until this new version is released with proper documentation
                #[allow(deprecated)]
                egui::CentralPanel::default()
                    .frame(egui::Frame::none().inner_margin(0))
                    .show(context, |ui| {
                        let mut snapshot = ui::pane::StateSnapshot {
                            pending_events: &mut self.pending_events,
                            project: &mut self.project,
                            rename_state: &mut self.rename_state,
                            errors: &self.errors,
                        };

                        snapshot.ui(
                            ui,
                            &mut self.inspector_tree_pane,
                            &mut self.viewport_tree_pane,
                        );
                    });
            });

        self.errors.clear();

        if let Err(error) = self.handle_events() {
            self.errors.push(SourcedError::new_unknown(error));
        }

        self.tick_objects(dt);

        if let Err(error) = self.scene.render(&self.device, &mut encoder, &self.project) {
            self.errors.push(SourcedError::new_unknown(error));
        }

        self.egui_renderer.render_egui_frame(
            &frame,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            &screen_descriptor,
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
            _ => {}
        }
    }

    fn tick_objects(&mut self, dt: std::time::Duration) {
        let mut tracker = RecreateTracker::new();

        let view = &mut TextureCreationContext {
            dimensions: &self.project.dimensions,
            device: &self.device,
            queue: &self.queue,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.textures, view));

        let view = &mut TextureViewCreationContext {
            textures: &self.project.textures,
            egui_renderer: &mut self.egui_renderer,
            device: &self.device,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.texture_views, view));

        let view = &mut ViewportCreationContext {
            texture_views: &self.project.texture_views,
            egui_renderer: &mut self.egui_renderer,
            device: &self.device,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.viewports, view));

        let view = &mut CameraCreationContext {
            dimensions: &self.project.dimensions,
            dt,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.cameras, view));

        tracker.recreate_storage(&mut self.project.samplers, &mut &self.device);

        let view = &mut UniformCreationContext {
            cameras: &self.project.cameras,
            device: &self.device,
            queue: &self.queue,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.uniforms, view));

        let view = &mut BindGroupCreationContext {
            uniforms: &self.project.uniforms,
            texture_views: &self.project.texture_views,
            samplers: &self.project.samplers,
            device: &self.device,
        };
        self.errors
            .extend(tracker.recreate_storage(&mut self.project.bind_groups, view));
    }

    fn handle_events(&mut self) -> AppResult<()> {
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
                        ProjectResourceId::Viewport(_id) => todo!(),
                        ProjectResourceId::Texture(_id) => todo!(),
                    };

                    self.inspector_tree_pane.add_pane(pane);
                }
                StateEvent::OpenViewport(viewport_id) => {
                    self.viewport_tree_pane
                        .add_pane(ViewportPane { viewport_id });
                }
                StateEvent::CreateUniform => {
                    const DEFAULT_NAME: &str = "Uniform";

                    let uniform =
                        project::uniform::Uniform::new(&self.device, DEFAULT_NAME, vec![])?;

                    let uniform_id = self.project.uniforms.register(uniform);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Uniform(uniform_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteUniform(id) => {
                    self.project.uniforms.unregister(id);
                }
                StateEvent::StartRename(rename_target) => {
                    let current_name = match rename_target {
                        RenameTarget::BindGroup(id) => self.project.label(id),
                        RenameTarget::Viewport(id) => self.project.label(id),
                        RenameTarget::Shader(id) => self.project.label(id),
                        RenameTarget::Camera(id) => self.project.label(id),
                        RenameTarget::Dimension(id) => self.project.label(id),
                        RenameTarget::Sampler(id) => self.project.label(id),
                        RenameTarget::TextureView(id) => self.project.label(id),
                        RenameTarget::Uniform(id) => self.project.label(id),
                        RenameTarget::UniformField(id, index) => self
                            .project
                            .uniforms
                            .get(id)
                            .ok()
                            .and_then(|uniform| uniform.get_field(index))
                            .map(|field| field.label()),
                    };

                    if let Some(current_name) = current_name {
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
                    match rename_target {
                        RenameTarget::Uniform(id) => {
                            if let Ok(uniform) = self.project.uniforms.get_mut(id) {
                                uniform.label = new_name;
                            }
                        }
                        RenameTarget::UniformField(id, index) => {
                            if let Ok(uniform) = self.project.uniforms.get_mut(id) {
                                uniform.set_field_label(index, new_name);
                            }
                        }
                        RenameTarget::BindGroup(id) => {
                            if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                                bind_group.label = new_name;
                            }
                        }
                        RenameTarget::Viewport(id) => {
                            if let Ok(viewport) = self.project.viewports.get_mut(id) {
                                viewport.label = new_name;
                            }
                        }
                        RenameTarget::Shader(id) => {
                            if let Ok(shader) = self.project.shaders.get_mut(id) {
                                shader.label = new_name;
                            }
                        }
                        RenameTarget::Camera(id) => {
                            if let Ok(camera) = self.project.cameras.get_mut(id) {
                                camera.label = new_name;
                            }
                        }
                        RenameTarget::Dimension(id) => {
                            if let Ok(dimension) = self.project.dimensions.get_mut(id) {
                                dimension.label = new_name;
                            }
                        }
                        RenameTarget::Sampler(id) => {
                            if let Ok(sampler) = self.project.samplers.get_mut(id) {
                                sampler.set_label(new_name);
                            }
                        }
                        RenameTarget::TextureView(texture_view_id) => {
                            if let Ok(texture_view) =
                                self.project.texture_views.get_mut(texture_view_id)
                            {
                                texture_view.set_label(new_name);
                            }
                        }
                    }
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
                StateEvent::DeleteUniformField(id, index) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(id) {
                        uniform.remove_field(index);
                    }
                }
                StateEvent::ReorderUniformField(uniform_id, drag_update) => {
                    if let Ok(uniform) = self.project.uniforms.get_mut(uniform_id) {
                        uniform.reorder_field(drag_update.from, drag_update.to);
                    }
                }
                StateEvent::CreateBindGroup => {
                    const DEFAULT_NAME: &str = "Bind Group";

                    let bind_group = project::bindgroup::BindGroup::new(
                        &self.project,
                        &self.device,
                        DEFAULT_NAME.to_string(),
                        vec![],
                    )?;

                    let bind_group_id = self.project.bind_groups.register(bind_group);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::BindGroup(bind_group_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteBindGroup(bind_group_id) => {
                    self.project.bind_groups.unregister(bind_group_id);
                }
                StateEvent::CreateBindGroupEntry(id, resource) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.add_entry(BindGroupEntry::new(resource));
                    }
                }
                StateEvent::DeleteBindGroupEntry(id, index) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.remove_entry(index);
                    }
                }
                StateEvent::UpdateBindGroupEntry(id, index, resource) => {
                    if let Ok(bind_group) = self.project.bind_groups.get_mut(id) {
                        bind_group.update_entry(index, BindGroupEntry::new(resource));
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

                                let is_owner = self
                                    .dimension_owners
                                    .get(viewport.dimension_id)
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
                                        self.project.dimensions.get_mut(viewport.dimension_id)
                                    {
                                        dimension.size = size;
                                    }
                                }
                            }
                            ViewportEvent::Focus => {
                                // read the comment in the event above for more context
                                self.dimension_owners
                                    .insert(viewport.dimension_id, viewport_id);
                                if let Some(ui_size) = viewport.requested_ui_size {
                                    if let Ok(dimension) =
                                        self.project.dimensions.get_mut(viewport.dimension_id)
                                    {
                                        dimension.size = ui_size;
                                    }
                                }
                            }
                            ViewportEvent::Scroll { delta_y_px } => {
                                if let Ok(camera) =
                                    self.project.cameras.get_mut(viewport.controls_camera_id)
                                {
                                    camera.input_mut().handle_scroll_pixels(delta_y_px);
                                }
                            }
                            ViewportEvent::Drag { mouse_dx, mouse_dy } => {
                                if let Ok(camera) =
                                    self.project.cameras.get_mut(viewport.controls_camera_id)
                                {
                                    camera.input_mut().handle_mouse(mouse_dx, mouse_dy);
                                }
                            }
                            ViewportEvent::KeyboardKeys { keyboard_state } => {
                                if let Ok(camera) =
                                    self.project.cameras.get_mut(viewport.controls_camera_id)
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
                StateEvent::DeleteCamera(camera_id) => {
                    self.project.cameras.unregister(camera_id);
                }
                StateEvent::CreateDimension => {
                    const DEFAULT_NAME: &str = "Dimension";

                    let dimension = Dimension {
                        label: DEFAULT_NAME.to_string(),
                        size: Size2d::new(1920, 1080),
                    };
                    let dimension_id = self.project.dimensions.register(dimension);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Dimension(dimension_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteDimension(dimension_id) => {
                    self.project.dimensions.unregister(dimension_id);
                }
                StateEvent::CreateSampler => {
                    const DEFAULT_NAME: &str = "Sampler";

                    let sampler = Sampler::new(
                        &self.device,
                        DEFAULT_NAME.to_string(),
                        SamplerSpec::default(),
                    )?;
                    let sampler_id = self.project.samplers.register(sampler);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Sampler(sampler_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteSampler(sampler_id) => {
                    self.project.samplers.unregister(sampler_id);
                }
            }
        }
        Ok(())
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
