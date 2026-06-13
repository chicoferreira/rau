use egui::Context;
use itertools::Itertools;

use crate::{
    error::AppError,
    file::file_system::ProjectFileSystem,
    project::{Project, ResourceId},
    ui::pane::StateSnapshot,
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

const ERROR_PANEL_ID: &str = "error_panel_expanded";

fn panel_id() -> egui::Id {
    egui::Id::new(ERROR_PANEL_ID)
}

fn is_open(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(panel_id()).unwrap_or(false))
}

fn set_open(ctx: &Context, open: bool) {
    ctx.data_mut(|d| d.insert_temp(panel_id(), open));
}

fn toggle_open(ctx: &Context) {
    let current = is_open(ctx);
    set_open(ctx, !current);
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    let mut show_error_list = is_open(ui.ctx());

    let errors = state.runtime_project.iter_errors().collect_vec();

    if errors.is_empty() && show_error_list {
        set_open(ui.ctx(), false);
        show_error_list = false;
    }

    let is_rebuilding = state.runtime_project.is_rebuilding();

    egui::Panel::bottom("status_panel")
        .resizable(false)
        .show_inside(ui, |ui| {
            status_bar_content(
                ui,
                &errors,
                is_rebuilding,
                state.backend,
                &state.file_storage.file_system,
            );
        });

    if show_error_list && !errors.is_empty() {
        egui::Panel::bottom("error_list_panel")
            .resizable(true)
            .default_size(200.0)
            .min_size(80.0)
            .show_inside(ui, |ui| {
                error_list_content(ui, state.project, &mut state.event_queue, &errors);
            });
    }
}

fn status_bar_content(
    ui: &mut egui::Ui,
    errors: &[(ResourceId, &AppError)],
    is_rebuilding: bool,
    backend: wgpu::Backend,
    project_file_system: &ProjectFileSystem,
) {
    ui.horizontal(|ui| {
        if errors.is_empty() {
            ui.label(egui::RichText::new("No errors").color(ui.visuals().weak_text_color()));
        } else {
            let error_count = errors.len();
            let label = format!(
                "{} error{}",
                error_count,
                if error_count == 1 { "" } else { "s" }
            );
            let btn =
                egui::Button::new(egui::RichText::new(label).color(ui.visuals().error_fg_color))
                    .frame(false);
            if ui
                .add(btn)
                .on_hover_text("Toggle error list")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                toggle_open(ui.ctx());
            }
        }

        if is_rebuilding {
            ui.add(egui::Spinner::new().size(ui.text_style_height(&egui::TextStyle::Body)))
                .on_hover_text("Rebuilding resources...");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            renderer_status_ui(ui, backend);
            ui.separator();
            storage_status_ui(ui, project_file_system);
        });
    });
}

fn storage_status_ui(ui: &mut egui::Ui, project_file_system: &ProjectFileSystem) {
    match project_file_system {
        ProjectFileSystem::Ephemeral(_) => {
            ui.colored_label(ui.visuals().warn_fg_color, "Temporary Storage")
                .on_hover_ui(|ui| {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        "Changes won't be saved after closing this project.",
                    );
                });
        }
        #[cfg(not(target_arch = "wasm32"))]
        ProjectFileSystem::Native(file_system) => {
            let response = ui.colored_label(ui.visuals().weak_text_color(), "Persistent Storage");
            response.on_hover_text(file_system.root().as_ref().display().to_string());
        }

        #[cfg(target_arch = "wasm32")]
        ProjectFileSystem::IndexedDb(file_system) => {
            let response = ui.colored_label(ui.visuals().weak_text_color(), "Persistent Storage");
            let hover_text = format!("IndexedDB with database {}", file_system.database_name());
            response.on_hover_text(hover_text);
        }
    }
}

fn renderer_status_ui(ui: &mut egui::Ui, backend: wgpu::Backend) {
    let backend_str = match backend {
        wgpu::Backend::Noop => "Noop",
        wgpu::Backend::Vulkan => "Vulkan",
        wgpu::Backend::Metal => "Metal",
        wgpu::Backend::Dx12 => "DirectX 12",
        wgpu::Backend::Gl => "GL (limited)",
        wgpu::Backend::BrowserWebGpu => "WebGPU",
    };

    let text = format!("Backend: {}", backend_str);

    if backend == wgpu::Backend::Gl {
        ui.colored_label(ui.visuals().warn_fg_color, text)
            .on_hover_ui(|ui| {
                let text = r#"GL support is limited.

Some features, including compute shaders, will not work.
Please enable WebGPU for the full renderer feature set."#;

                ui.colored_label(ui.visuals().warn_fg_color, text);
            });
    } else {
        ui.colored_label(ui.visuals().weak_text_color(), text);
    }
}

fn error_list_content(
    ui: &mut egui::Ui,
    project: &Project,
    event_queue: &mut EventQueue<StateEvent>,
    errors: &[(ResourceId, &AppError)],
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for (id, error) in errors {
                let id = *id;
                ui.horizontal_wrapped(|ui| {
                    let source_label = project.label(id).unwrap_or("Unknown");

                    let label_text = egui::RichText::new(format!("@{}", source_label))
                        .strong()
                        .underline()
                        .color(ui.visuals().warn_fg_color);

                    let response = ui.add(egui::Button::new(label_text).frame(false));
                    if response
                        .on_hover_text("Click to inspect source")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        event_queue.inspect_resource(id);
                    }

                    ui.label(
                        egui::RichText::new(error.to_string())
                            .monospace()
                            .color(ui.visuals().error_fg_color),
                    );
                });

                ui.separator();
            }
        });
}
