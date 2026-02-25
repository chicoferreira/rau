use egui::{Label, Sense, Ui, Widget};

use crate::{
    project::uniform::{self, UniformField, UniformFieldKind, UniformFieldType, UniformId},
    state::StateEvent,
    ui::{
        components::{
            color_edit::color_edit_rgba, edit_number_array::ui_edit_array_h,
            renameable_label::renameable_label,
        },
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
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

                ui.label(
                    egui::RichText::new(uniform.label.clone())
                        .size(ui.text_style_height(&egui::TextStyle::Body) + 2.0)
                        .strong(),
                );

                let (total_size, _) = data.layout();
                ui.horizontal(|ui| {
                    ui.label("Total size");
                    ui.strong(format!("{total_size} bytes"));
                });

                ui.add_space(6.0);

                for (index, field) in data.fields.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.add(ui_uniform_label(
                            uniform_id,
                            field,
                            index,
                            self.pending_events,
                            self.rename_state,
                        ));

                        let id = ui.id().with("field").with(index).with(uniform_id);
                        updated |= ui_edit_uniform_field(ui, &mut field.ty, id)
                    });
                }

                ui.add_space(6.0);

                ui.menu_button("Add Uniform", |ui| {
                    for kind in UniformFieldKind::all() {
                        if ui.button(kind.label()).clicked() {
                            let event = StateEvent::CreateUniformField(uniform_id, kind);
                            self.pending_events.push(event);
                        }
                    }
                });

                if updated {
                    let event = StateEvent::UpdateUniform(uniform_id);
                    self.pending_events.push(event);
                }
            });
        });
    }
}

fn ui_uniform_label(
    uniform_id: UniformId,
    field: &UniformField,
    field_index: usize,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
) -> impl Widget {
    move |ui: &mut Ui| {
        let rename_target = RenameTarget::UniformField(uniform_id, field_index);
        ui.horizontal(|ui| {
            ui.add(renameable_label(
                Label::new(&field.name)
                    .selectable(false)
                    .sense(Sense::click()),
                pending_events,
                rename_state,
                rename_target.clone(),
            ))
            .context_menu(|ui| {
                if ui.button("Delete Field").clicked() {
                    pending_events.push(StateEvent::DeleteUniformField(uniform_id, field_index));
                    ui.close();
                }
                if ui.button("Rename Field").clicked() {
                    pending_events.push(StateEvent::StartRename(rename_target));
                    ui.close();
                }
            });

            let (align, size) = field.ty.layout();
            ui_uniform_type_label(ui, field.ty.kind(), align, size);
        })
        .response
    }
}

fn ui_uniform_type_label(ui: &mut Ui, kind: UniformFieldKind, align: usize, size: usize) {
    egui::Popup::from_toggle_button_response(
        &ui.label(egui::RichText::new(kind.label()).weak())
            .on_hover_cursor(egui::CursorIcon::PointingHand),
    )
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
            let wgsl_type_label = kind.wgsl_type_label();
            ui.label(egui::RichText::new(wgsl_type_label).monospace().strong());
        });
    });
}

fn ui_edit_uniform_field(
    ui: &mut egui::Ui,
    ty: &mut uniform::UniformFieldType,
    id: egui::Id,
) -> bool {
    match ty {
        uniform::UniformFieldType::Vec4f(vec4) => ui_edit_array_h(ui, vec4),
        uniform::UniformFieldType::Vec3f(vec3) => ui_edit_array_h(ui, vec3),
        uniform::UniformFieldType::Vec2f(vec2) => ui_edit_array_h(ui, vec2),
        uniform::UniformFieldType::Mat4x4f(mat4) => {
            let mut updated = false;
            egui::Grid::new(id).show(ui, |ui| {
                for row in mat4.iter_mut() {
                    updated |= ui_edit_array_h(ui, row);
                    ui.end_row();
                }
            });
            updated
        }
        uniform::UniformFieldType::Rgba(color) => color_edit_rgba(ui, color),
        uniform::UniformFieldType::Rgb(color) => {
            egui::color_picker::color_edit_button_rgb(ui, color).changed()
        }
    }
}

impl UniformFieldType {
    fn kind(&self) -> UniformFieldKind {
        match self {
            UniformFieldType::Vec2f(_) => UniformFieldKind::Vec2f,
            UniformFieldType::Vec3f(_) => UniformFieldKind::Vec3f,
            UniformFieldType::Vec4f(_) => UniformFieldKind::Vec4f,
            UniformFieldType::Rgb(_) => UniformFieldKind::Rgb,
            UniformFieldType::Rgba(_) => UniformFieldKind::Rgba,
            UniformFieldType::Mat4x4f(_) => UniformFieldKind::Mat4x4f,
        }
    }
}
