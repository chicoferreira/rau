use egui::{Frame, RichText, Stroke, TextStyle, Ui, Widget};

pub fn hint<R>(add_contents: impl FnOnce(&mut Ui) -> R) -> impl Widget {
    |ui: &mut egui::Ui| {
        Frame::new()
            .fill(ui.visuals().panel_fill)
            .stroke(Stroke::new(1.0, ui.visuals().text_color()))
            .corner_radius(4.0)
            .inner_margin(6.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // Trick so we don't have to add spaces in the text below:
                    let width =
                        ui.fonts_mut(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), ' '));
                    ui.spacing_mut().item_spacing.x = width;

                    ui.label(RichText::new("💡 Hint:").strong());
                    add_contents(ui);
                })
            })
            .response
    }
}
