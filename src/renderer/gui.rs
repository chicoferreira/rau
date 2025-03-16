use crate::renderer::Renderer;
use egui::color_picker::Alpha;
use egui::RichText;
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

            ui.heading("Textures");
            for (index, texture) in renderer.renderer_project.textures.iter().enumerate() {
                ui.collapsing(RichText::from(&texture.name).strong(), |ui| {
                    if let Some(&texture_id) = renderer.renderer_project.textures_egui.get(index) {
                        let sized_texture =
                            egui::load::SizedTexture::new(texture_id, egui::vec2(100.0, 100.0));
                        ui.image(sized_texture);
                    } else {
                        ui.label("Not registered");
                    }
                });
            }

            ui.heading("Models");
            for model in renderer.renderer_project.models.iter() {
                ui.collapsing(RichText::from(&model.name).strong(), |ui| {
                    for (index, mesh) in model.meshes.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Mesh {}", index));
                            ui.label(
                                RichText::from(format!("{} elements", mesh.num_elements)).weak(),
                            );
                        });
                    }
                });
            }
        });
}
