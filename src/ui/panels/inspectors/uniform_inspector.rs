use egui::{Label, Sense, Ui, Widget};
use strum::IntoEnumIterator;

use crate::{
    project::{
        UniformId,
        uniform::{
            self, CameraField, UniformField, UniformFieldKind, UniformFieldSource,
            UniformFieldSourceKind,
        },
    },
    state::StateEvent,
    ui::{
        components::{
            color_edit::color_edit_rgba,
            data_display::{drag_value_widget, label_widget, ui_edit_array},
            renameable_label::renameable_label,
        },
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
    },
};

impl StateSnapshot<'_> {
    pub fn uniform_inspector_ui(&mut self, uniform_id: UniformId, ui: &mut egui::Ui) {
        let Some(uniform) = self.project.uniforms.get_mut(uniform_id) else {
            ui.label("Uniform couldn't be found.");
            return;
        };

        let data = &mut uniform.data;

        let (total_size, _) = data.layout();
        ui.horizontal(|ui| {
            ui.label("Total size");
            ui.strong(format!("{total_size} bytes"));
        });

        ui.add_space(6.0);

        let response =
            egui_dnd::dnd(ui, "uniform").show(data.fields.iter_mut(), |ui, item, handle, state| {
                handle.show_drag_cursor_on_hover(false).ui(ui, |ui| {
                    ui.push_id(state.index, |ui| {
                        ui_uniform_field(
                            ui,
                            uniform_id,
                            state.index,
                            item,
                            self.pending_events,
                            self.rename_state,
                        );
                    });
                });
            });

        if let Some(update) = response.final_update() {
            self.pending_events
                .push(StateEvent::ReorderUniformField(uniform_id, update));
        }

        ui.add_space(6.0);

        ui.add(ui_add_uniform(uniform_id, &mut self.pending_events));
    }
}

fn ui_uniform_field(
    ui: &mut Ui,
    uniform_id: UniformId,
    index: usize,
    field: &mut UniformField,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let rename_target = RenameTarget::UniformField(uniform_id, index);
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
                    pending_events.push(StateEvent::DeleteUniformField(uniform_id, index));
                    ui.close();
                }
                if ui.button("Rename Field").clicked() {
                    pending_events.push(StateEvent::StartRename(rename_target));
                    ui.close();
                }
            });

            let mut source = field.source.kind();
            let source_before = source;

            egui::ComboBox::from_id_salt(ui.id().with("selection"))
                .selected_text(source.to_string())
                .show_ui(ui, |ui| {
                    ui.label("User Defined");
                    for kind in UniformFieldKind::iter() {
                        ui.selectable_value(
                            &mut source,
                            UniformFieldSourceKind::UserDefined(kind),
                            kind.to_string(),
                        );
                    }
                    ui.separator();
                    ui.label("Camera");
                    for kind in CameraField::iter() {
                        ui.selectable_value(
                            &mut source,
                            UniformFieldSourceKind::Camera(kind),
                            kind.to_string(),
                        );
                    }
                });

            if source != source_before {
                let event = StateEvent::UpdateUniformFieldSource(uniform_id, index, source);
                pending_events.push(event);
            }

            ui_uniform_type_label(ui, field.kind());
        });

        ui.horizontal(|ui| match &mut field.source {
            UniformFieldSource::UserDefined(data) => {
                ui_uniform_field_data(ui, data, drag_value_widget);
            }
            _ => {
                ui.horizontal(|ui| {
                    ui_uniform_field_data(ui, &mut field.last_data, label_widget);
                });
            }
        });
    });
}

fn ui_uniform_type_label(ui: &mut Ui, kind: UniformFieldKind) {
    let (align, size) = kind.layout();
    egui::Popup::from_toggle_button_response(
        &ui.label(egui::RichText::new(kind.to_string()).weak())
            .on_hover_cursor(egui::CursorIcon::Help),
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

fn ui_uniform_field_data(
    ui: &mut egui::Ui,
    data: &mut uniform::UniformFieldData,
    number_widget: fn(&mut egui::Ui, &mut f32),
) {
    match data {
        uniform::UniformFieldData::Vec4f(vec4) => ui_edit_array(ui, vec4, number_widget),
        uniform::UniformFieldData::Vec3f(vec3) => ui_edit_array(ui, vec3, number_widget),
        uniform::UniformFieldData::Vec2f(vec2) => ui_edit_array(ui, vec2, number_widget),
        uniform::UniformFieldData::Mat4x4f(mat4) => {
            egui::Grid::new(ui.id().with("fieldmat4")).show(ui, |ui| {
                for row in mat4.iter_mut() {
                    ui_edit_array(ui, row, number_widget);
                    ui.end_row();
                }
            });
        }
        uniform::UniformFieldData::Rgba(color) => {
            color_edit_rgba(ui, color);
        }
        uniform::UniformFieldData::Rgb(color) => {
            egui::color_picker::color_edit_button_rgb(ui, color);
        }
    }
}

fn ui_add_uniform(uniform_id: UniformId, pending_events: &mut Vec<StateEvent>) -> impl Widget {
    move |ui: &mut Ui| {
        ui.menu_button("Add Uniform", |ui| {
            ui.label("User Defined");
            for kind in UniformFieldKind::iter() {
                if ui.button(kind.to_string()).clicked() {
                    let event = StateEvent::CreateUniformField(
                        uniform_id,
                        UniformFieldSourceKind::UserDefined(kind),
                    );
                    pending_events.push(event);
                }
            }
            ui.separator();
            ui.label("Camera");
            for kind in CameraField::iter() {
                if ui.button(kind.to_string()).clicked() {
                    let event = StateEvent::CreateUniformField(
                        uniform_id,
                        UniformFieldSourceKind::Camera(kind),
                    );
                    pending_events.push(event);
                }
            }
        })
        .response
    }
}
