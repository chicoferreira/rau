//! Window fullscreen toggle.

/// Whether the root viewport is currently fullscreen.
pub fn is_fullscreen(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().fullscreen).unwrap_or(false)
}

/// Toggle the window between fullscreen and windowed.
pub fn toggle(ctx: &egui::Context) {
    let target = !is_fullscreen(ctx);
    ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(target));
}

/// Toggle fullscreen when F11 is pressed. Call once per frame near the top of the UI
/// so the shortcut works regardless of which screen (main menu or workspace) is showing.
pub fn handle_shortcut(ctx: &egui::Context) {
    if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
        toggle(ctx);
    }
}
