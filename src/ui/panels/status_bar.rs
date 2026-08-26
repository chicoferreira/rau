use egui_phosphor::regular;

use crate::{
    file::file_system::ProjectFileSystem,
    ui::{components::field, pane::StateSnapshot, panels::error_panel::ErrorPanel},
    utils::fps::FrameTimeTracker,
};

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui, error_panel: &mut ErrorPanel) {
    puffin::profile_function!();

    let is_rebuilding = state.runtime_project.is_rebuilding();

    ui.horizontal(|ui| {
        error_panel.status_indicator(ui);

        if is_rebuilding {
            field::spinner(ui).on_hover_text("Rebuilding resources...");
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            renderer_status_ui(ui, state.backend);
            ui.separator();
            storage_status_ui(ui, &state.file_storage.file_system);
            ui.separator();
            vsync_status_ui(ui, state);
            ui.separator();
            frame_time_ui(ui, state.frame_time);
        });
    });
}

fn frame_time_ui(ui: &mut egui::Ui, frame_time: &FrameTimeTracker) {
    let frame_time_ms = frame_time.displayed_ms();
    let fps = if frame_time_ms > 0.0 {
        1000.0 / frame_time_ms
    } else {
        0.0
    };

    let profiler_open = puffin::are_scopes_on();
    let caret = match profiler_open {
        true => regular::CARET_DOWN,
        false => regular::CARET_UP,
    };

    let text = format!("{frame_time_ms:.1}ms ({fps:.1} FPS) {caret}");

    let button = egui::Button::new(egui::RichText::new(text).color(ui.visuals().weak_text_color()))
        .frame(false);

    let response = ui.add(button).on_hover_ui(|ui| {
        ui.strong("Frame Profiler");
        ui.label("Breaks each frame down into the scopes that make it up.");
        ui.label(
            egui::RichText::new(match profiler_open {
                true => "Click to close the profiler.",
                false => "Click to open the profiler.",
            })
            .weak(),
        );
    });

    if response.clicked() {
        puffin::set_scopes_on(!profiler_open);
    }
}

fn vsync_status_ui(ui: &mut egui::Ui, state: &mut StateSnapshot) {
    let vsync_on = !matches!(
        state.present_mode,
        wgpu::PresentMode::AutoNoVsync | wgpu::PresentMode::Immediate
    );

    let title = match state.present_mode {
        wgpu::PresentMode::AutoVsync => "VSync",
        wgpu::PresentMode::AutoNoVsync | wgpu::PresentMode::Immediate => "Immediate",
        wgpu::PresentMode::Fifo => "FIFO",
        wgpu::PresentMode::FifoRelaxed => "FIFO Relaxed",
        wgpu::PresentMode::Mailbox => "Mailbox",
    };

    let button =
        egui::Button::new(egui::RichText::new(title).color(ui.visuals().weak_text_color()))
            .frame(false);
    let response = ui.add(button).on_hover_ui(|ui| {
        ui.strong(title);
        ui.label(match vsync_on {
            true => "Renders frames at the monitor's refresh rate to prevent tearing.",
            false => "Renders frames as fast as possible for the lowest latency, but may cause screen tearing.",
        });
        ui.label(egui::RichText::new("Click to toggle VSync.").weak());
    });

    if response.clicked() {
        let next = match vsync_on {
            true => wgpu::PresentMode::AutoNoVsync,
            false => wgpu::PresentMode::AutoVsync,
        };
        state.app_event_queue.set_present_mode(next);
    }
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

    let text = format!("{} Backend", backend_str);

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
