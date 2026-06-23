use slotmap::SecondaryMap;

use crate::{
    app::AppEvent,
    error::AppResult,
    file::{
        file_storage::FileStorage,
        file_system::{AppFileSystem, ProjectFileSystemTrait},
        identifier::ProjectSource,
    },
    project::{
        DimensionId, ModelId, Project, ResourceId, ResourceKind, RuntimeProject, TextureId,
        ViewportId,
        paths::FilePath,
        render::{self, PresentationRender},
        resource::{
            bindgroup::BindGroupCreationContext,
            camera::CameraCreationContext,
            compute_pass,
            model::{ModelCreationContext, TextureType},
            render_pipeline,
            shader::ShaderCreationContext,
            texture::TextureCreationContext,
            texture_view::TextureViewCreationContext,
            uniform::UniformCreationContext,
        },
        save::ProjectSaveState,
        sync::SyncTracker,
    },
    ui::{
        self,
        components::{derive_model_material_modal::MaterialBindGroupsModal, tiles::TreePane},
        panels::{
            error_panel::ErrorPanel, inspector_pane::InspectorPane, viewport_pane::ViewportPane,
        },
        rename::{RenameState, RenameTarget},
        size::Size2d,
    },
    utils::{
        async_job::AsyncJob,
        derive::{derive_texture_from_material_path, derive_texture_view},
        event_queue::EventQueue,
        fps::FrameTimeTracker,
        key::KeyboardState,
        texture_capture::TextureCaptures,
        wgpu_utils::create_command_encoder,
    },
};

pub struct Workspace {
    project: Project,
    runtime_project: RuntimeProject,
    tracker: SyncTracker,
    file_storage: FileStorage,
    project_save_state: ProjectSaveState,
    rename_state: Option<ui::rename::RenameState>,
    event_queue: EventQueue<StateEvent>,
    inspector_tree_pane: TreePane<InspectorPane>,
    viewport_tree_pane: TreePane<ViewportPane>,
    error_panel: ErrorPanel,
    dimension_owners: SecondaryMap<DimensionId, ViewportId>,
    /// The model `.mtl` files each model depended on at its last successful load.
    /// Needed because model `.mtl` files are not part of the runtime state and
    /// need to be tracked separately to support reloading the `.obj` model when a
    /// `.mtl` file changes.
    mtl_dependencies: SecondaryMap<ModelId, Vec<FilePath>>,
    elapsed: instant::Duration,
    texture_captures: TextureCaptures,
    toasts: egui_notify::Toasts,
    material_bind_groups_modal: Option<MaterialBindGroupsModal>,
}

pub struct AppContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub egui_renderer: &'a mut ui::renderer::EguiRenderer,
    pub downlevel_flags: wgpu::DownlevelFlags,
    pub dt: instant::Duration,
}

#[derive(Debug, Clone)]
pub enum ViewportEvent {
    Resize { size: Size2d },
    Scroll { scroll_delta: f32 },
    Drag { mouse_dx: f32, mouse_dy: f32 },
    KeyboardKeys { keyboard_state: KeyboardState },
    Focus,
}

#[derive(Debug, Clone)]
pub enum StateEvent {
    ViewportEvent(ViewportId, ViewportEvent),
    OpenFile(FilePath),
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
    DeleteFolder(FilePath),
    ImportFile(FilePath),
    ReplaceFile(FilePath),
    #[cfg(target_arch = "wasm32")]
    DownloadFile(FilePath),
    MoveFileSystemEntry {
        old_path: FilePath,
        new_path: FilePath,
    },
    SetMainViewport(ViewportId),
    DownloadTextureImage(TextureId),
    OpenMaterialBindGroupsModal(ModelId),
    /// Create a new texture view backed by the given texture.
    DeriveTextureView(TextureId),
    /// Create a new texture sourced from the given material image path.
    DeriveTextureFromPath {
        path: FilePath,
        texture_type: TextureType,
    },
}

impl Workspace {
    pub async fn open_project_and_save_files(
        app_fs: AppFileSystem,
        source: ProjectSource,
        files: Vec<(FilePath, Vec<u8>)>,
    ) -> AppResult<Self> {
        if !files.is_empty() {
            app_fs.ensure_project_can_be_created(source.clone()).await?;
        }

        let (file_system, file_watcher) = app_fs.mount_project(source.clone()).await?;

        let file_storage = FileStorage::new(source.clone(), file_system, file_watcher);

        for (file_path, data) in files {
            file_storage.file_system.write(&file_path, data).await?;
        }

        let workspace = Self::open_project(file_storage).await?;
        if let Err(error) = app_fs.remember_project(source).await {
            log::error!("Failed to remember project: {error}");
        }

        Ok(workspace)
    }

    async fn open_project(file_storage: FileStorage) -> AppResult<Self> {
        let project_bytes = file_storage.read(&FilePath::project_json()).await?;
        let project: Project = serde_json::from_slice(&project_bytes)?;

        let inspector_tree_pane = TreePane::new("inspector");
        let mut viewport_tree_pane = TreePane::new("viewport");

        if let Some(viewport_id) = project.presentation.main_viewport() {
            viewport_tree_pane.add_pane(ViewportPane { viewport_id });
        }

        let project_save_state = ProjectSaveState::new(&project);

        Ok(Self {
            rename_state: None,
            event_queue: EventQueue::default(),
            inspector_tree_pane,
            viewport_tree_pane,
            error_panel: ErrorPanel::default(),
            project,
            runtime_project: RuntimeProject::default(),
            tracker: SyncTracker::default(),
            file_storage,
            project_save_state,
            dimension_owners: Default::default(),
            mtl_dependencies: SecondaryMap::default(),
            elapsed: instant::Duration::ZERO,
            texture_captures: TextureCaptures::default(),
            toasts: egui_notify::Toasts::default(),
            material_bind_groups_modal: None,
        })
    }

    pub fn render(&mut self, ctx: &mut AppContext) {
        self.elapsed += ctx.dt;
        self.handle_events();
        self.project_save_state
            .tick(&self.project, &mut self.file_storage);

        self.file_storage.tick(&mut self.tracker);

        self.texture_captures.tick(
            &self.project,
            &self.runtime_project,
            ctx.device,
            ctx.queue,
            &mut self.toasts,
        );

        for (_, camera) in self.project.cameras.list_mut() {
            camera.update(ctx.dt);
        }

        let resources_changed = self.tick_objects(ctx);

        // Compute dispatches get their own encoder, which is *always* submitted. The viewport
        // render below uses a separate, droppable encoder: when a render pass bails out on a
        // still-rebuilding or errored resource, that whole encoder is discarded without being
        // finished. If the compute dispatches shared it, a dropped viewport frame would silently
        // drop them too, and an `OnChange` pass, which only dispatches on the frame its inputs
        // change, would miss that one dispatch and never re-run, leaving its output stuck.
        let mut compute_encoder = create_command_encoder(ctx.device, "Compute Encoder");
        let mut compute_ctx = render::ComputeDispatchContext {
            compute_passes: &self.project.compute_passes,
            runtime_compute_passes: &mut self.runtime_project.compute_passes,
            runtime_bind_groups: &self.runtime_project.bind_groups,
            compute_accumulators: &mut self.runtime_project.compute_accumulators,
            tracker: &self.tracker,
            dt: ctx.dt,
        };
        self.project
            .presentation
            .dispatch_computes(&mut compute_encoder, &mut compute_ctx);

        ctx.queue.submit(std::iter::once(compute_encoder.finish()));

        // Now that the compute dispatch has consumed this frame's change set, it can be cleared.
        self.tracker.clear_changes();

        let snapshot = self.project.snapshot();
        if !self
            .runtime_project
            .poll_presentation_errors(snapshot, resources_changed)
        {
            return;
        }

        let mut render_ctx = render::RenderContext {
            models: &self.project.models,
            render_pipelines: &self.project.render_pipelines,
            render_passes: &self.project.render_passes,
            runtime_models: &self.runtime_project.models,
            runtime_bind_groups: &self.runtime_project.bind_groups,
            runtime_texture_views: &self.runtime_project.texture_views,
            runtime_render_pipelines: &self.runtime_project.render_pipelines,
            render_pass_errors: &mut self.runtime_project.render_pass_errors,
        };

        // The viewport render uses a separate, droppable encoder: if a render pass bails out
        // because a resource is still rebuilding, we drop the whole encoder without finishing it.
        //
        // This avoids the flicker that would otherwise occur because of the pass LoadOp::Clear
        // because of the `begin_render_pass` call.
        let mut viewport_encoder = create_command_encoder(ctx.device, "Viewport Render Encoder");

        let presentation = &self.project.presentation;
        match presentation.render(&mut viewport_encoder, &mut render_ctx) {
            Ok(true) => {
                ctx.queue.submit([viewport_encoder.finish()]);
            }
            Ok(false) => {} // A resource is still pending, drop the encoder without submitting
            Err(error) => {
                let snapshot = self.project.snapshot();
                let error = PresentationRender::Errored { error, snapshot };
                self.runtime_project.presentation_render = error;
            }
        }
    }

    pub fn project_name(&self) -> &str {
        self.file_storage.project_source().project_name()
    }

    pub fn on_frame_submitted(&mut self, job: AsyncJob<AppResult<()>>) {
        let current_snapshot = self.project.snapshot();
        self.runtime_project
            .on_frame_submitted(current_snapshot, job);
    }

    pub fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        backend: wgpu::Backend,
        present_mode: wgpu::PresentMode,
        frame_time: &FrameTimeTracker,
        app_event_queue: &mut EventQueue<AppEvent>,
    ) {
        self.toasts.show(ui.ctx());

        let mut snapshot = ui::pane::StateSnapshot {
            event_queue: &mut self.event_queue,
            app_event_queue,
            project: &mut self.project,
            runtime_project: &mut self.runtime_project,
            rename_state: &mut self.rename_state,
            file_storage: &mut self.file_storage,
            backend,
            present_mode,
            frame_time,
        };

        snapshot.ui(
            ui,
            &mut self.inspector_tree_pane,
            &mut self.viewport_tree_pane,
            &mut self.error_panel,
        );

        self.material_bind_groups_modal_ui(ui);
    }

    fn material_bind_groups_modal_ui(&mut self, ui: &mut egui::Ui) {
        let Some(modal) = self.material_bind_groups_modal.as_mut() else {
            return;
        };

        let project = &mut self.project;
        let keep_open = modal.show(ui, project, &self.runtime_project, &mut self.toasts);

        if !keep_open {
            self.material_bind_groups_modal = None;
        }
    }

    fn handle_events(&mut self) {
        for event in self.event_queue.drain() {
            log::debug!("Handling event {event:?}");
            match event {
                StateEvent::OpenFile(file_path) => {
                    let pane = InspectorPane::File(file_path);
                    self.inspector_tree_pane.add_pane(pane);
                }
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
                        ResourceId::RenderPipeline(id) => InspectorPane::RenderPipeline(id),
                        ResourceId::RenderPass(id) => InspectorPane::RenderPass(id),
                        ResourceId::Presentation(id) => InspectorPane::Presentation(id),
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
                StateEvent::DeleteFolder(file_path) => {
                    self.file_storage.delete_folder_in_background(file_path);
                }
                StateEvent::ImportFile(parent_path) => {
                    self.file_storage.import_file_in_background(parent_path);
                }
                StateEvent::ReplaceFile(file_path) => {
                    self.file_storage.replace_file_in_background(file_path);
                }
                #[cfg(target_arch = "wasm32")]
                StateEvent::DownloadFile(file_path) => {
                    self.file_storage.download_file_in_background(file_path);
                }
                StateEvent::MoveFileSystemEntry { old_path, new_path } => {
                    self.file_storage
                        .move_path_in_background(old_path, new_path);
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
                                viewport.set_requested_ui_size(Some(size));

                                if let Some(dimension_id) = viewport.dimension_id() {
                                    let is_owner = self
                                        .dimension_owners
                                        .get(dimension_id)
                                        .is_none_or(|&owner| owner == viewport_id);

                                    // relevant issue: https://github.com/chicoferreira/rau/issues/8
                                    //
                                    // this is a bit hacky, but we want the camera to resize immediately if this
                                    // viewport is the owner of the dimension or the viewport has no owners, otherwise
                                    // we'll wait until it gets focused (handled in the event below). this avoids the
                                    // problem of fighting when there are two viewports with different sizes for the same dimension.
                                    // this way, only one of them (the owner) will control the dimension size.
                                    if is_owner
                                        && let Ok(dimension) =
                                            self.project.dimensions.get_mut(dimension_id)
                                    {
                                        dimension.set_size(size);
                                    }
                                }
                            }
                            ViewportEvent::Focus => {
                                // read the comment in the event above for more context
                                if let Some(dimension_id) = viewport.dimension_id() {
                                    self.dimension_owners.insert(dimension_id, viewport_id);
                                    if let Some(ui_size) = viewport.requested_ui_size()
                                        && let Ok(dimension) =
                                            self.project.dimensions.get_mut(dimension_id)
                                    {
                                        dimension.set_size(ui_size);
                                    }
                                }
                            }
                            ViewportEvent::Scroll { scroll_delta } => {
                                if let Some(camera_id) = viewport.controls_camera_id()
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.input_mut().handle_scroll(scroll_delta);
                                }
                            }
                            ViewportEvent::Drag { mouse_dx, mouse_dy } => {
                                if let Some(camera_id) = viewport.controls_camera_id()
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.input_mut().handle_mouse(mouse_dx, mouse_dy);
                                }
                            }
                            ViewportEvent::KeyboardKeys { keyboard_state } => {
                                if let Some(camera_id) = viewport.controls_camera_id()
                                    && let Ok(camera) = self.project.cameras.get_mut(camera_id)
                                {
                                    camera.handle_keyboard(&keyboard_state);
                                }
                            }
                        }
                    }
                }
                StateEvent::SetMainViewport(viewport_id) => {
                    let presentation = &mut self.project.presentation;
                    presentation.set_main_viewport(Some(viewport_id));
                }
                StateEvent::DownloadTextureImage(texture_id) => {
                    self.texture_captures.request(texture_id);
                }
                StateEvent::OpenMaterialBindGroupsModal(model_id) => {
                    if let Some(modal) = MaterialBindGroupsModal::open(
                        &self.project,
                        &self.runtime_project,
                        self.file_storage.files(),
                        &mut self.toasts,
                        model_id,
                    ) {
                        self.material_bind_groups_modal = Some(modal);
                    }
                }
                StateEvent::DeriveTextureView(texture_id) => {
                    if self.project.textures.get(texture_id).is_ok() {
                        let texture_view_id = derive_texture_view(&mut self.project, texture_id);
                        self.inspector_tree_pane
                            .add_pane(InspectorPane::TextureView(texture_view_id));
                    }
                }
                StateEvent::DeriveTextureFromPath { path, texture_type } => {
                    let texture_id =
                        derive_texture_from_material_path(&mut self.project, path, texture_type);
                    self.inspector_tree_pane
                        .add_pane(InspectorPane::Texture(texture_id));
                }
            }
        }
    }

    /// Syncs every resource for this frame, returning whether any runtime resource changed.
    fn tick_objects(&mut self, ctx: &mut AppContext) -> bool {
        self.tracker.sync_storage(
            &mut self.project.dimensions,
            &mut self.runtime_project.dimensions,
            &mut (),
        );

        let view = &mut TextureCreationContext {
            dimensions: &self.project.dimensions,
            device: ctx.device,
            queue: ctx.queue,
            file_storage: &self.file_storage,
            downlevel_flags: ctx.downlevel_flags,
        };
        self.tracker.sync_storage(
            &mut self.project.textures,
            &mut self.runtime_project.textures,
            view,
        );

        let view = &mut TextureViewCreationContext {
            textures: &self.project.textures,
            egui_renderer: ctx.egui_renderer,
            device: ctx.device,
            textures_runtime: &mut self.runtime_project.textures,
            downlevel_flags: ctx.downlevel_flags,
        };
        self.tracker.sync_storage(
            &mut self.project.texture_views,
            &mut self.runtime_project.texture_views,
            view,
        );

        let view = &mut CameraCreationContext {
            dimensions: &self.project.dimensions,
            dt: ctx.dt,
        };
        self.tracker.sync_storage(
            &mut self.project.cameras,
            &mut self.runtime_project.cameras,
            view,
        );

        self.tracker.sync_storage(
            &mut self.project.samplers,
            &mut self.runtime_project.samplers,
            &mut ctx.device,
        );

        let view = &mut UniformCreationContext {
            cameras: &self.project.cameras,
            device: ctx.device,
            queue: ctx.queue,
            cameras_runtime: &mut self.runtime_project.cameras,
            time: self.elapsed.as_secs_f32(),
        };
        self.tracker.sync_storage(
            &mut self.project.uniforms,
            &mut self.runtime_project.uniforms,
            view,
        );

        let view = &mut BindGroupCreationContext {
            device: ctx.device,
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
            device: ctx.device,
            queue: ctx.queue,
            file_storage: &self.file_storage,
            runtime_bind_groups: &self.runtime_project.bind_groups,
            mtl_dependencies: &mut self.mtl_dependencies,
        };
        self.tracker.sync_storage(
            &mut self.project.models,
            &mut self.runtime_project.models,
            view,
        );

        let view = &mut ShaderCreationContext {
            device: ctx.device,
            file_storage: &self.file_storage,
        };
        self.tracker.sync_storage(
            &mut self.project.shaders,
            &mut self.runtime_project.shaders,
            view,
        );

        let view = &mut render_pipeline::Context {
            device: ctx.device,
            runtime_shaders: &mut self.runtime_project.shaders,
            runtime_bind_groups: &self.runtime_project.bind_groups,
            models: &self.project.models,
            runtime_models: &self.runtime_project.models,
        };
        self.tracker.sync_storage(
            &mut self.project.render_pipelines,
            &mut self.runtime_project.render_pipelines,
            view,
        );

        let view = &mut compute_pass::Context {
            device: ctx.device,
            runtime_shaders: &mut self.runtime_project.shaders,
            runtime_bind_groups: &mut self.runtime_project.bind_groups,
        };
        self.tracker.sync_storage(
            &mut self.project.compute_passes,
            &mut self.runtime_project.compute_passes,
            view,
        );

        // Note: the tracker is *not* cleared here. `dispatch_computes` runs after
        // `tick_objects` returns and still needs this frame's change set to decide
        // which `OnChange` passes to dispatch; the caller clears it afterwards.
        self.tracker.has_resource_changes()
    }
}
