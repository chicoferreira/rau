use std::sync::Arc;

use slotmap::KeyData;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    camera::{Camera, CameraInput},
    project::{
        self, BindGroupId, ShaderId, UniformId, ViewportId,
        bindgroup::BindGroupProjectView,
        texture::TextureProjectView,
        texture_view::TextureViewProjectView,
        uniform::{UniformData, UniformField, UniformFieldSourceKind, UniformProjectView},
        viewport::ViewportContext,
    },
    rebuild::RebuildTracker,
    resources, scene,
    ui::{
        self,
        panels::{
            inspector_pane::{InspectorPane, InspectorTreePane},
            viewport_pane::ViewportTreePane,
        },
        rename::{RenameState, RenameTarget},
    },
};

#[derive(Debug, Clone)]
pub enum ViewportEvent {
    Resize {
        size: ui::Size2d,
    },
    Scroll {
        delta_y_px: f32,
    },
    Drag {
        mouse_dx: f32,
        mouse_dy: f32,
    },
    Keyboard {
        key_code: winit::keyboard::KeyCode,
        element_state: winit::event::ElementState,
    },
}

#[derive(Debug, Clone)]
pub enum StateEvent {
    ViewportEvent(ViewportId, ViewportEvent),
    InspectUniform(UniformId),
    InspectBindGroup(BindGroupId),
    OpenViewport(ViewportId),
    CreateUniform,
    DeleteUniform(UniformId),
    CreateUniformField(UniformId, UniformFieldSourceKind),
    UpdateUniformFieldSource(UniformId, usize, UniformFieldSourceKind),
    DeleteUniformField(UniformId, usize),
    StartRename(RenameTarget),
    CancelRename,
    ApplyRename(RenameTarget, String),
    InspectShader(ShaderId),
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
    inspector_tree_pane: InspectorTreePane,
    viewport_tree_pane: ViewportTreePane,
    camera_input: CameraInput,
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

        let camera = Camera::new(
            (0.0, 5.0, 10.0),
            cgmath::Deg(-90.0),
            cgmath::Deg(-20.0),
            size.width(),
            size.height(),
            cgmath::Deg(45.0),
            0.1,
            100.0,
        );

        let camera_input = CameraInput::new(4.0, 0.4);

        let mut project = project::Project::new(camera);

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

        let inspector_tree_pane = InspectorTreePane::default();
        let mut viewport_tree_pane = ViewportTreePane::default();

        viewport_tree_pane.add_viewport(scene.output_viewport_id);

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
            rename_state: None,
            pending_events: vec![],
            inspector_tree_pane,
            viewport_tree_pane,
            camera_input,
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
                        };

                        snapshot.ui(
                            ui,
                            &mut self.inspector_tree_pane,
                            &mut self.viewport_tree_pane,
                        );
                    });
            });

        self.handle_events();

        self.camera_input
            .update_camera(&mut self.project.camera, dt);

        for (_, uniform) in self.project.uniforms.list_mut() {
            uniform.update(
                UniformProjectView {
                    camera: &self.project.camera,
                },
                &self.device,
                &self.queue,
            );
        }

        self.tick_objects();

        self.scene.update(&mut self.project, dt);

        self.scene.render(&mut encoder, &self.project);

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
                    self.pending_events.push(StateEvent::ViewportEvent(
                        ViewportId::from(KeyData::from_ffi(0)), // TODO: fix me
                        ViewportEvent::Keyboard {
                            key_code,
                            element_state,
                        },
                    ));
                }
            },
            _ => {}
        }
    }

    fn tick_objects(&mut self) {
        let mut recreate_list = RebuildTracker::new(&self.device, &self.queue);

        let view = &mut TextureProjectView {
            viewports: &self.project.viewports,
            dimensions: &self.project.dimensions,
        };
        recreate_list.recreate_storage(&mut self.project.textures, view);

        let view = &mut TextureViewProjectView {
            textures: &self.project.textures,
        };
        recreate_list.recreate_storage(&mut self.project.texture_views, view);

        let view = &mut ViewportContext {
            texture_views: &self.project.texture_views,
            egui_renderer: &mut self.egui_renderer,
        };
        recreate_list.recreate_storage(&mut self.project.viewports, view);

        let view = &mut BindGroupProjectView {
            uniforms: &self.project.uniforms,
            texture_views: &self.project.texture_views,
            samplers: &self.project.samplers,
        };
        recreate_list.recreate_storage(&mut self.project.bind_groups, view);
    }

    fn handle_events(&mut self) {
        for event in self.pending_events.drain(..) {
            match event {
                StateEvent::InspectUniform(id) => {
                    self.inspector_tree_pane
                        .add_inspector_pane(InspectorPane::Uniform(id));
                }
                StateEvent::OpenViewport(viewport_id) => {
                    self.viewport_tree_pane.add_viewport(viewport_id);
                }
                StateEvent::CreateUniform => {
                    const DEFAULT_NAME: &str = "Uniform";

                    let uniform = project::uniform::Uniform::new(
                        &self.device,
                        DEFAULT_NAME,
                        UniformData::default(),
                    );

                    let uniform_id = self.project.uniforms.register(uniform);

                    self.rename_state = Some(RenameState {
                        target: RenameTarget::Uniform(uniform_id),
                        current_label: DEFAULT_NAME.to_string(),
                    });
                }
                StateEvent::DeleteUniform(id) => {
                    self.project.uniforms.unregister(id);
                }
                StateEvent::InspectBindGroup(bind_group_id) => self
                    .inspector_tree_pane
                    .add_inspector_pane(InspectorPane::BindGroup(bind_group_id)),
                StateEvent::StartRename(rename_target) => {
                    let current_name = match rename_target {
                        RenameTarget::Uniform(uniform_id) => self
                            .project
                            .uniforms
                            .get(uniform_id)
                            .map(|u| u.label.clone()),
                        RenameTarget::UniformField(uniform_id, index) => self
                            .project
                            .uniforms
                            .get(uniform_id)
                            .map(|uniform| uniform.data.fields.get(index))
                            .flatten()
                            .map(|field| field.name.clone()),
                        RenameTarget::BindGroup(bind_group_id) => self
                            .project
                            .bind_groups
                            .get(bind_group_id)
                            .map(|b| b.label.clone()),
                        RenameTarget::Viewport(viewport_id) => self
                            .project
                            .viewports
                            .get(viewport_id)
                            .map(|t| t.label.clone()),
                        RenameTarget::Shader(shader_id) => {
                            self.project.shaders.get(shader_id).map(|s| s.label.clone())
                        }
                    };

                    if let Some(current_name) = current_name {
                        self.rename_state = Some(RenameState {
                            target: rename_target,
                            current_label: current_name,
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
                            if let Some(uniform) = self.project.uniforms.get_mut(id) {
                                uniform.label = new_name;
                            }
                        }
                        RenameTarget::UniformField(id, index) => {
                            if let Some(uniform) = self.project.uniforms.get_mut(id) {
                                if let Some(field) = uniform.data.fields.get_mut(index) {
                                    field.name = new_name;
                                }
                            }
                        }
                        RenameTarget::BindGroup(id) => {
                            if let Some(bind_group) = self.project.bind_groups.get_mut(id) {
                                bind_group.label = new_name;
                            }
                        }
                        RenameTarget::Viewport(id) => {
                            if let Some(viewport) = self.project.viewports.get_mut(id) {
                                viewport.label = new_name;
                            }
                        }
                        RenameTarget::Shader(id) => {
                            if let Some(shader) = self.project.shaders.get_mut(id) {
                                shader.label = new_name;
                            }
                        }
                    }
                }
                StateEvent::CreateUniformField(id, uniform_field_kind) => {
                    if let Some(uniform) = self.project.uniforms.get_mut(id) {
                        const DEFAULT_NAME: &str = "Field";

                        let index = uniform.data.fields.len();
                        uniform.data.fields.push(UniformField::new_from_kind(
                            DEFAULT_NAME,
                            uniform_field_kind,
                        ));

                        self.rename_state = Some(RenameState {
                            target: RenameTarget::UniformField(id, index),
                            current_label: DEFAULT_NAME.to_string(),
                        });
                    }
                }
                StateEvent::UpdateUniformFieldSource(
                    uniform_id,
                    index,
                    uniform_field_source_kind,
                ) => {
                    if let Some(uniform) = self.project.uniforms.get_mut(uniform_id) {
                        let field = &mut uniform.data.fields[index];
                        let name = field.name.clone();
                        *field = UniformField::new_from_kind(name, uniform_field_source_kind);
                    }
                }
                StateEvent::DeleteUniformField(id, index) => {
                    if let Some(uniform) = self.project.uniforms.get_mut(id) {
                        let fields = &mut uniform.data.fields;
                        if index < fields.len() {
                            fields.remove(index);
                        }
                    }
                }
                StateEvent::InspectShader(shader_id) => {
                    self.inspector_tree_pane
                        .add_inspector_pane(InspectorPane::Shader(shader_id));
                }
                StateEvent::ViewportEvent(viewport_id, viewport_event) => {
                    let _viewport = self.project.viewports.get_mut(viewport_id);
                    let camera = &mut self.project.camera;

                    match viewport_event {
                        ViewportEvent::Resize { size } => {
                            camera.resize(size);
                            self.scene.resize(size, &mut self.project);
                        }
                        ViewportEvent::Scroll { delta_y_px } => {
                            self.camera_input.handle_scroll_pixels(delta_y_px);
                        }
                        ViewportEvent::Drag { mouse_dx, mouse_dy } => {
                            self.camera_input.handle_mouse(mouse_dx, mouse_dy);
                        }
                        ViewportEvent::Keyboard {
                            key_code,
                            element_state,
                        } => {
                            self.camera_input.handle_keyboard(key_code, element_state);
                        }
                    }
                }
            }
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
