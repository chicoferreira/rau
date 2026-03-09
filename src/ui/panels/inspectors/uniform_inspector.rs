use egui::{Label, RichText, Sense, Ui, Widget};
use strum::IntoEnumIterator;

use crate::{
    project::{
        UniformId,
        uniform::{
            self, CameraFieldSource, UniformField, UniformFieldKind, UniformFieldSource,
            UniformFieldSourceKind,
        },
    },
    state::StateEvent,
    ui::{
        components::{color_edit::color_edit_rgba, renameable_label::renameable_label},
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

        for (index, field) in data.fields.iter_mut().enumerate() {
            if index != 0 {
                ui.add_space(5.0);
            }
            ui.push_id(index, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui_uniform_field_label(
                            ui,
                            uniform_id,
                            field,
                            index,
                            self.pending_events,
                            self.rename_state,
                        );

                        let mut source = field.source.kind();
                        let source_before = source;
                        ui.add(ui_uniform_field_source(&mut source));
                        if source != source_before {
                            let event =
                                StateEvent::UpdateUniformFieldSource(uniform_id, index, source);
                            self.pending_events.push(event);
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
            });
        }

        ui.add_space(6.0);

        ui.add(ui_add_uniform(uniform_id, &mut self.pending_events));
    }
}

fn ui_uniform_field_label(
    ui: &mut Ui,
    uniform_id: UniformId,
    field: &UniformField,
    field_index: usize,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
) {
    let rename_target = RenameTarget::UniformField(uniform_id, field_index);
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
}

fn ui_uniform_field_source(source: &mut UniformFieldSourceKind) -> impl Widget {
    move |ui: &mut Ui| {
        egui::ComboBox::from_id_salt(ui.id().with("selection"))
            .selected_text(source.to_string())
            .show_ui(ui, |ui| {
                ui.label("User Defined");
                for kind in UniformFieldKind::iter() {
                    ui.selectable_value(
                        source,
                        UniformFieldSourceKind::UserDefined(kind),
                        kind.to_string(),
                    );
                }
                ui.separator();
                ui.label("Camera");
                for kind in CameraFieldSource::iter() {
                    ui.selectable_value(
                        source,
                        UniformFieldSourceKind::Camera(kind),
                        kind.to_string(),
                    );
                }
            })
            .response
    }
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

fn ui_edit_array<const N: usize>(
    ui: &mut egui::Ui,
    array: &mut [f32; N],
    widget: impl Fn(&mut egui::Ui, &mut f32),
) {
    for value in array.iter_mut() {
        widget(ui, value);
    }
}

fn drag_value_widget(ui: &mut egui::Ui, value: &mut f32) {
    ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(2));
}

fn label_widget(ui: &mut egui::Ui, value: &mut f32) {
    ui.label(RichText::new(format!("{value:.2}")).weak());
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
            for kind in CameraFieldSource::iter() {
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
