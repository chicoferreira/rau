use egui_phosphor::regular;

use crate::{
    project::{PresentationId, ProjectResource, ResourceKind, paths::FilePath},
    ui::{components::resource_icons, pane::StateSnapshot},
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

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("Rau", |ui| rau_menu(state, ui));
        ui.menu_button("Project", |ui| project_menu(state, ui));
        ui.menu_button("Create", |ui| create_menu(state, ui));
        ui.menu_button("View", |ui| view_menu(state, ui));
    });
}

fn rau_menu(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    use crate::built_info;

    ui.label(
        egui::RichText::new(concat!("Rau ", env!("CARGO_PKG_VERSION")))
            .strong()
            .size(14.0),
    );
    ui.hyperlink_to(
        format!("{} Open source code", regular::GITHUB_LOGO),
        env!("CARGO_PKG_REPOSITORY"),
    );

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

    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.separator();
        if ui.button("Quit").clicked() {
            state.app_event_queue.quit();
        }
    }
}

fn info_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.add(egui::Label::new(egui::RichText::new(key).weak()).selectable(true));
    ui.add(egui::Label::new(egui::RichText::new(value).monospace()).selectable(true));
    ui.end_row();
}

fn project_menu(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    if ui.button("New File").clicked() {
        state
            .event_queue
            .add(StateEvent::CreateFile(FilePath::default()));
    }
    if ui.button("New Folder").clicked() {
        state
            .event_queue
            .add(StateEvent::CreateFolder(FilePath::default()));
    }
    if ui.button("Import File…").clicked() {
        state
            .event_queue
            .add(StateEvent::ImportFile(FilePath::default()));
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
