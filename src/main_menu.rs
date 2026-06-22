use crate::{
    StartupAction,
    app::{AppEvent, State},
    error::AppResult,
    file::{file_system::AppFileSystem, identifier::ProjectSource},
    project::{Project, paths::FilePath},
    ui::components::{
        create_project_modal::{
            CreateProjectModal, CreateProjectModalResponse, ProjectCreationSource,
        },
        main_menu::{
            featured_projects, menu_widgets, open_or_import_project::OpenOrImportProject,
            recent_projects::RecentProjectsState,
        },
        resource_icons,
    },
    utils::{self, async_job::AsyncJob, event_queue::EventQueue},
    workspace::Workspace,
};

use egui::{Color32, RichText};
use egui_phosphor::regular;
use std::task::Poll;

const CONTENT_MAX_WIDTH: f32 = 1180.0;
const CONTENT_MARGIN: f32 = 36.0;

#[derive(Default)]
pub struct MainMenu {
    toasts: egui_notify::Toasts,
    open_workspace_job: Option<AsyncJob<AppResult<Workspace>>>,
    open_or_import_project: OpenOrImportProject,
    recent_projects_state: RecentProjectsState,
    create_project_modal: Option<CreateProjectModal>,
    logo_texture: Option<egui::TextureHandle>,
}

impl MainMenu {
    pub fn with_startup_action(app_fs: AppFileSystem, startup_action: StartupAction) -> Self {
        let mut main_menu = Self::default();

        match startup_action {
            StartupAction::MainMenu => {}
            StartupAction::OpenProject { project_id } => {
                main_menu.open_project(app_fs, ProjectSource::Persistent(project_id), vec![])
            }
            StartupAction::CreateProject { source, creation } => match creation {
                ProjectCreationSource::Empty => match Project::default().serialize() {
                    Ok(bytes) => {
                        let default_files = vec![(FilePath::project_json(), bytes)];
                        main_menu.open_project(app_fs, source, default_files);
                    }
                    Err(err) => {
                        toasts_log_error!(main_menu.toasts, "Failed to serialize project: {err:?}");
                    }
                },
                ProjectCreationSource::Github(github) => {
                    let modal = CreateProjectModal::from_cli(&app_fs, source, github);
                    main_menu.create_project_modal = Some(modal);
                }
            },
        }

        main_menu
    }

    pub fn render_ui(&mut self, ui: &mut egui::Ui, app_fs: &AppFileSystem) {
        self.toasts.show(ui.ctx());

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                content_container(ui, |ui| {
                    ui.add_enabled_ui(!self.should_disable_ui(), |ui| {
                        ui.add_space(28.0);
                        self.header(ui);
                        ui.add_space(28.0);

                        if let Some(project_id) = self.recent_projects_state.render_ui(ui, app_fs) {
                            let source = ProjectSource::Persistent(project_id);
                            self.open_project(app_fs.clone(), source, vec![]);
                        }

                        ui.add_space(28.0);
                        if let Some(project) = featured_projects::render_ui(ui) {
                            self.create_project_modal =
                                Some(CreateProjectModal::from_featured_project(project));
                        }
                        ui.add_space(40.0);
                    });
                });
            });

        if let Some(modal) = &mut self.create_project_modal
            && let Some(response) = modal.render_ui(ui, app_fs, &mut self.toasts)
        {
            match response {
                CreateProjectModalResponse::Create { source, files } => {
                    self.open_project(app_fs.clone(), source, files);
                    self.create_project_modal = None;
                }
                CreateProjectModalResponse::Close => {
                    self.create_project_modal = None;
                }
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let logo = self
                .logo_texture
                .get_or_insert_with(|| load_logo(ui.ctx()))
                .clone();

            ui.add(egui::Image::from_texture((logo.id(), [80.0, 80.0].into())));

            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.add_space(5.0); // Add some vertical space to vertically center the text

                ui.label(
                    RichText::new("rau")
                        .size(34.0)
                        .variation("wght", 600.0)
                        .strong(),
                );
                ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak());
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 10.0;

                ui.add_enabled_ui(!self.should_disable_ui(), |ui| {
                    self.open_or_import_project.render_ui(ui);
                });

                let new_project = menu_widgets::primary_action_button(
                    ui,
                    resource_icons::monochrome_icon_text(
                        ui,
                        regular::PLUS,
                        Color32::WHITE,
                        "New Project",
                    ),
                );
                if new_project.clicked() {
                    self.create_project_modal =
                        Some(CreateProjectModal::new(ProjectCreationSource::Empty));
                }
            });
        });
    }

    fn open_project(
        &mut self,
        app_fs: AppFileSystem,
        source: ProjectSource,
        files: Vec<(FilePath, Vec<u8>)>,
    ) {
        let workspace_job = Workspace::open_project_and_save_files(app_fs, source, files);
        let workspace_job = AsyncJob::new(workspace_job);
        self.open_workspace_job = Some(AsyncJob::new(workspace_job));
    }

    pub fn render(
        &mut self,
        app_event_queue: &mut EventQueue<AppEvent>,
        app_file_system: &AppFileSystem,
    ) {
        self.recent_projects_state
            .tick(app_file_system, &mut self.toasts);

        if let Some(result) = self.open_or_import_project.tick() {
            match result {
                #[cfg(target_arch = "wasm32")]
                Ok(project_import) => {
                    let project_id = project_import.project_id;
                    let files = project_import.files;
                    let source = ProjectSource::Persistent(project_id);
                    self.open_project(app_file_system.clone(), source, files);
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ok(project_id) => {
                    let source = ProjectSource::Persistent(project_id);
                    self.open_project(app_file_system.clone(), source, vec![]);
                }
                Err(error) => {
                    toasts_log_error!(self.toasts, "Failed to pick project folder: {error:?}");
                    self.recent_projects_state.reload();
                }
            }
        }

        if let Some(job) = &mut self.open_workspace_job
            && let Poll::Ready(result) = job.try_resolve()
        {
            match result {
                Ok(workspace) => {
                    app_event_queue.add(AppEvent::SetState(State::Workspace(workspace)));
                }
                Err(error) => {
                    toasts_log_error!(self.toasts, "Failed to open workspace: {error:?}");
                    self.recent_projects_state.reload();
                }
            }
            self.open_workspace_job = None;
        }
    }

    fn should_disable_ui(&self) -> bool {
        self.open_workspace_job.is_some() || self.open_or_import_project.is_picker_opened()
    }
}

fn content_container<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let outer = ui.available_rect_before_wrap();
    let width = (outer.width() - 2.0 * CONTENT_MARGIN).clamp(0.0, CONTENT_MAX_WIDTH);
    let left = outer.left() + ((outer.width() - width) * 0.5).max(0.0);
    let rect = egui::Rect::from_min_max(
        egui::pos2(left, outer.top()),
        egui::pos2(left + width, f32::INFINITY),
    );

    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
        content,
    )
    .inner
}

fn load_logo(ctx: &egui::Context) -> egui::TextureHandle {
    let image = image::load_from_memory(utils::icon::LOGO_IMAGE_BYTES)
        .expect("Failed to decode logo")
        .into_rgba8();

    let (width, height) = image.dimensions();
    let color_image =
        egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], image.as_raw());

    ctx.load_texture("rau-logo", color_image, egui::TextureOptions::LINEAR)
}
