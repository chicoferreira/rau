use egui::TextEdit;

use crate::{project::shader::ShaderId, ui::pane::StateSnapshot};

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.take_available_space();
                    let shader = self.project.get_shader_mut(shader_id).unwrap();
                    
                    let mut no_wrap_layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
                            let text_color = ui.visuals().widgets.inactive.text_color();
                            let font_id = egui::TextStyle::Monospace.resolve(ui.style());
                            let mut layout_job = egui::text::LayoutJob::simple(
                                text.as_str().to_owned(),
                                font_id,
                                text_color,
                                f32::INFINITY,
                            );
                            layout_job.wrap.max_rows = usize::MAX;
                            ui.fonts_mut(|f| f.layout_job(layout_job))
                        };

                    TextEdit::multiline(&mut shader.source.as_ref())
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .layouter(&mut no_wrap_layouter)
                        .show(ui);
                });
        });
    }
}
