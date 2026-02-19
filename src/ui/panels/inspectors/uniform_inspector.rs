use crate::{
    project::uniform::{self, UniformId},
    ui::{
        components::{color_edit::color_edit_rgba, edit_number_array::ui_edit_array_h},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn uniform_inspector_ui(&mut self, uniform_id: UniformId, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                let Some(uniform) = self.project.get_uniform_mut(uniform_id) else {
                    ui.label("No uniforms registered.");
                    return;
                };

                let data = &mut uniform.data;
                let mut updated = false;

                let header_label = egui::RichText::new(uniform.label.clone())
                    .size(ui.text_style_height(&egui::TextStyle::Body) + 2.0)
                    .strong();

                egui::CollapsingHeader::new(header_label)
                    .default_open(true)
                    .show(ui, |ui| {
                        let (total_size, _) = data.layout();
                        ui.horizontal(|ui| {
                            ui.label("Total size");
                            ui.strong(format!("{total_size} bytes"));
                        });

                        ui.add_space(6.0);

                        for (index, field) in data.fields.iter_mut().enumerate() {
                            let (align, size) = field.ty.layout();
                            let type_label = uniform_field_type_label(&field.ty);

                            ui.horizontal(|ui| {
                                ui.label(&field.name);
                                let response = ui
                                    .add(
                                        egui::Label::new(egui::RichText::new(type_label).weak())
                                            .sense(egui::Sense::click()),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                                egui::Popup::from_toggle_button_response(&response)
                                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                    .show(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Size");
                                            ui.strong(format!("{size} bytes"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Alignment");
                                            ui.strong(format!("{align} bytes"));
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("WGSL type");
                                            // TODO: make this syntax highlight
                                            let wgsl_type_label =
                                                uniform_wgsl_type_label(&field.ty);
                                            ui.label(
                                                egui::RichText::new(wgsl_type_label)
                                                    .monospace()
                                                    .strong(),
                                            );
                                        });
                                    });
                            });

                            match &mut field.ty {
                                uniform::UniformFieldType::Vec4f(vec4) => {
                                    updated |= ui_edit_array_h(ui, vec4)
                                }
                                uniform::UniformFieldType::Vec3f(vec3) => {
                                    updated |= ui_edit_array_h(ui, vec3)
                                }
                                uniform::UniformFieldType::Vec2f(vec2) => {
                                    updated |= ui_edit_array_h(ui, vec2);
                                }
                                uniform::UniformFieldType::Mat4x4f(mat4) => {
                                    egui::Grid::new(format!("uniform_{uniform_id:?}_mat4_{index}"))
                                        .show(ui, |ui| {
                                            for row in mat4.iter_mut() {
                                                updated |= ui_edit_array_h(ui, row);
                                                ui.end_row();
                                            }
                                        });
                                }
                                uniform::UniformFieldType::Rgba(color) => {
                                    updated |= color_edit_rgba(ui, color);
                                }
                                uniform::UniformFieldType::Rgb(color) => {
                                    updated |= egui::color_picker::color_edit_button_rgb(ui, color)
                                        .changed();
                                }
                            }
                            ui.add_space(8.0);
                        }
                    });

                if updated {
                    uniform.upload(self.queue);
                }
            });
        });
    }
}

fn uniform_field_type_label(ty: &uniform::UniformFieldType) -> &'static str {
    match ty {
        uniform::UniformFieldType::Vec2f(_) => "Vec2f",
        uniform::UniformFieldType::Vec3f(_) => "Vec3f",
        uniform::UniformFieldType::Vec4f(_) => "Vec4f",
        uniform::UniformFieldType::Rgb(_) => "Rgb",
        uniform::UniformFieldType::Rgba(_) => "Rgba",
        uniform::UniformFieldType::Mat4x4f(_) => "Mat4x4f",
    }
}

fn uniform_wgsl_type_label(ty: &uniform::UniformFieldType) -> &'static str {
    match ty {
        uniform::UniformFieldType::Vec2f(_) => "vec2<f32>",
        uniform::UniformFieldType::Vec3f(_) => "vec3<f32>",
        uniform::UniformFieldType::Vec4f(_) => "vec4<f32>",
        uniform::UniformFieldType::Rgb(_) => "vec3<f32>",
        uniform::UniformFieldType::Rgba(_) => "vec4<f32>",
        uniform::UniformFieldType::Mat4x4f(_) => "mat4x4<f32>",
    }
}
