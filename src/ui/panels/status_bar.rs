use crate::{
    file::file_system::ProjectFileSystem,
    ui::{components::inspector, pane::StateSnapshot, panels::error_panel::ErrorPanel},
};

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui, error_panel: &mut ErrorPanel) {
    let is_rebuilding = state.runtime_project.is_rebuilding();
    let backend = state.backend;
    let file_system = &state.file_storage.file_system;

    ui.horizontal(|ui| {
        error_panel.status_indicator(ui);

        if is_rebuilding {
            inspector::spinner(ui).on_hover_text("Rebuilding resources...");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            renderer_status_ui(ui, backend);
            ui.separator();
            storage_status_ui(ui, file_system);
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
