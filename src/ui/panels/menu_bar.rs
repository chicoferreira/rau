use egui_phosphor::regular;

use crate::{
    project::{PresentationId, ProjectResource, ResourceKind, paths::FilePath},
    ui::{components::resource_icons, fullscreen, pane::StateSnapshot},
    workspace::StateEvent,
};

const CREATABLE_RESOURCES: &[(ResourceKind, &str)] = &[
    (ResourceKind::RenderPass, "Render Pass"),
    (ResourceKind::ComputePass, "Compute Pass"),
    (ResourceKind::RenderPipeline, "Render Pipeline"),
    (ResourceKind::Shader, "Shader"),
    (ResourceKind::BindGroup, "Bind Group"),
    (ResourceKind::Uniform, "Uniform"),
    (ResourceKind::Texture, "Texture"),
    (ResourceKind::TextureView, "Texture View"),
    (ResourceKind::Sampler, "Sampler"),
    (ResourceKind::Model, "Model"),
    (ResourceKind::Camera, "Camera"),
    (ResourceKind::Viewport, "Viewport"),
    (ResourceKind::Dimension, "Dimension"),
];

pub const SAVE_SHORTCUT: egui::KeyboardShortcut =
    egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    puffin::profile_function!();

    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("Rau", |ui| rau_menu(state, ui));
        ui.menu_button("Project", |ui| project_menu(state, ui));
        ui.menu_button("Create", |ui| create_menu(state, ui));
        ui.menu_button("View", |ui| view_menu(state, ui));
    });
}

fn rau_menu(_state: &mut StateSnapshot, ui: &mut egui::Ui) {
    use crate::built_info;

    ui.label(
        egui::RichText::new(concat!("Rau ", env!("CARGO_PKG_VERSION")))
            .strong()
            .size(14.0),
    );
    let repository = env!("CARGO_PKG_REPOSITORY");
    if ui
        .link(format!("{} Open source code", regular::GITHUB_LOGO))
        .on_hover_text(repository)
        .clicked()
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab(repository));
    }

    ui.separator();

    let commit = match built_info::GIT_COMMIT_HASH_SHORT {
        Some(hash) if matches!(built_info::GIT_DIRTY, Some(true)) => format!("{hash} (dirty)"),
        Some(hash) => hash.to_owned(),
        None => "unknown".to_owned(),
    };

    egui::Grid::new("rau_build_info")
        .num_columns(2)
        .spacing([12.0, 2.0])
        .show(ui, |ui| {
            info_row(ui, "Commit", &commit);
            info_row(ui, "Built", built_info::BUILT_TIME_UTC);
            if let Some(ci) = built_info::CI_PLATFORM {
                info_row(ui, "CI", ci);
            }
            info_row(ui, "Profile", built_info::PROFILE);
            info_row(ui, "Target", built_info::TARGET);
            info_row(ui, "Compiler", built_info::RUSTC_VERSION);
        });

    ui.separator();

    theme_menu(ui);

    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.separator();
        if ui.button("Quit").clicked() {
            _state.app_event_queue.quit();
        }
    }
}

fn theme_menu(ui: &mut egui::Ui) {
    const THEMES: [(egui::ThemePreference, &str, &str); 3] = [
        (
            egui::ThemePreference::System,
            regular::DESKTOP_TOWER,
            "System",
        ),
        (egui::ThemePreference::Light, regular::SUN, "Light"),
        (egui::ThemePreference::Dark, regular::MOON, "Dark"),
    ];

    let current = ui.options(|options| options.theme_preference);

    ui.menu_button(format!("{} Theme", regular::PALETTE), |ui| {
        for (preference, icon, label) in THEMES {
            if ui
                .radio(current == preference, format!("{icon} {label}"))
                .clicked()
            {
                ui.ctx().set_theme(preference);
            }
        }
    });
}

fn info_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.add(egui::Label::new(egui::RichText::new(key).weak()).selectable(true));
    ui.add(egui::Label::new(egui::RichText::new(value).monospace()).selectable(true));
    ui.end_row();
}

fn project_menu(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    let mut emit_event = |event: StateEvent| state.event_queue.add(event);

    let save = egui::Button::new(format!("{} Save Now", regular::FLOPPY_DISK))
        .shortcut_text(ui.ctx().format_shortcut(&SAVE_SHORTCUT));
    if ui.add(save).clicked() {
        emit_event(StateEvent::SaveProject);
    }
    ui.label(egui::RichText::new("Changes are saved automatically.").weak());

    ui.separator();

    let new_file = ui.button(format!("{} New File", regular::FILE_PLUS));
    if new_file.clicked() {
        emit_event(StateEvent::CreateFile(FilePath::default()));
    }

    let new_folder = ui.button(format!("{} New Folder", regular::FOLDER_PLUS));
    if new_folder.clicked() {
        emit_event(StateEvent::CreateFolder(FilePath::default()));
    }

    let import_file = ui.button(format!("{} Import File…", regular::UPLOAD_SIMPLE));
    if import_file.clicked() {
        emit_event(StateEvent::ImportFile(FilePath::default()));
    }

    #[cfg(target_arch = "wasm32")]
    {
        let download_project = ui.button(format!("{} Download Project", regular::DOWNLOAD_SIMPLE));
        if download_project.clicked() {
            emit_event(StateEvent::DownloadProject);
        }
    }

    ui.separator();
    if ui.button("Close Project").clicked() {
        state.app_event_queue.close_project();
    }
}

fn create_menu(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    for &(kind, label) in CREATABLE_RESOURCES {
        let icon = resource_icons::resource_kind_icon(kind);
        let icon_text = resource_icons::icon_text(ui, icon, label);
        if ui.button(icon_text).clicked() {
            state.event_queue.add(StateEvent::CreateResource(kind));
        }
    }
}

fn view_menu(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    let viewport_icon = resource_icons::resource_kind_icon(ResourceKind::Viewport);

    let is_fullscreen = fullscreen::is_fullscreen(ui.ctx());
    let (icon, label) = if is_fullscreen {
        (regular::ARROWS_IN, "Exit Fullscreen")
    } else {
        (regular::ARROWS_OUT, "Enter Fullscreen")
    };
    let button = egui::Button::new(format!("{icon} {label}")).shortcut_text("F11");
    if ui.add(button).clicked() {
        fullscreen::toggle(ui.ctx());
    }

    ui.separator();

    if ui.button("Inspect Presentation").clicked() {
        state.event_queue.inspect_resource(PresentationId);
    }

    ui.separator();

    let main_viewport = state.project.presentation.main_viewport();
    let viewports: Vec<_> = state
        .project
        .viewports
        .list_sorted()
        .map(|(id, viewport)| (id, viewport.label().to_string()))
        .collect();

    if viewports.is_empty() {
        ui.add_enabled(false, egui::Button::new("No viewports — create one first"));
        return;
    }

    ui.menu_button("Open Viewport", |ui| {
        for (id, label) in &viewports {
            let icon_text = resource_icons::icon_text(ui, viewport_icon, label);
            if ui.button(icon_text).clicked() {
                state.event_queue.add(StateEvent::OpenViewport(*id));
            }
        }
    });

    ui.menu_button("Set Main Viewport", |ui| {
        for (id, label) in &viewports {
            let is_main = main_viewport == Some(*id);
            let icon_text = resource_icons::icon_text(ui, viewport_icon, label);
            if ui.radio(is_main, icon_text).clicked() && !is_main {
                state.event_queue.add(StateEvent::SetMainViewport(*id));
            }
        }
    });
}
