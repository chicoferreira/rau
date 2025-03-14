use crate::renderer::Renderer;
use egui::color_picker::Alpha;
use enum2egui::GuiInspect;

pub fn render_gui(renderer: &mut Renderer) {
    let context = &mut renderer.egui.context;
    egui::Window::new("Rau")
        .default_open(true)
        .show(context, |ui| {
            ui.heading("Viewport");
            ui.horizontal(|ui| {
                ui.label("Clear Color");
                let color = renderer.renderer_project.viewport_clear_color;
                let mut rgba = egui::Rgba::from_rgba_premultiplied(
                    color.r as f32,
                    color.g as f32,
                    color.b as f32,
                    color.a as f32,
                );
                if egui::color_picker::color_edit_button_rgba(ui, &mut rgba, Alpha::OnlyBlend)
                    .changed()
                {
                    renderer.renderer_project.viewport_clear_color = wgpu::Color {
                        r: rgba[0] as f64,
                        g: rgba[1] as f64,
                        b: rgba[2] as f64,
                        a: rgba[3] as f64,
                    };
                };
            });
            ui.heading("Camera");
            renderer.renderer_project.camera.ui_mut(ui);
        });
}
