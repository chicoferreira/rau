use egui::{Label, RichText, Sense, Ui};
use egui_dnd::DragDropItem;
use strum::IntoEnumIterator;

use crate::{
    project::{
        CameraId, UniformId,
        camera::Camera,
        storage::Storage,
        uniform::{
            self, UniformField, UniformFieldData, UniformFieldDataKind, UniformFieldSource,
            camera::CameraField,
        },
    },
    state::StateEvent,
    ui::{
        components::{
            color_edit::color_edit_rgba,
            data_display::{ui_array, ui_array_mut},
            hint::hint,
            renameable_label::renameable_label,
            selector::{AsWidgetText, ComboBoxExt},
        },
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum UniformFieldSourceKind {
    #[strum(to_string = "User Defined")]
    UserDefined,
    Camera,
}

impl UniformFieldSourceKind {
    pub fn from_source(source: &UniformFieldSource) -> Self {
        match source {
            UniformFieldSource::UserDefined(_) => Self::UserDefined,
            UniformFieldSource::Camera { .. } => Self::Camera,
        }
    }

    pub fn into_source(self) -> UniformFieldSource {
        match self {
            Self::UserDefined => UniformFieldSource::new_user_defined(UniformFieldData::from_kind(
                UniformFieldDataKind::Vec3f,
            )),
            Self::Camera => UniformFieldSource::new_camera_sourced(None, CameraField::Position),
        }
    }
}

impl StateSnapshot<'_> {
    pub fn uniform_inspector_ui(&mut self, uniform_id: UniformId, ui: &mut egui::Ui) {
        let Ok(uniform) = self.project.uniforms.get(uniform_id) else {
            ui.label("Uniform couldn't be found.");
            return;
        };

        let (total_size, _) = uniform.layout();
        ui.horizontal(|ui| {
            ui.label("Total size");
            ui.strong(format!("{total_size} bytes"));
        });

        ui.add_space(6.0);

        let mut ctx = UniformUiContext {
            pending_events: &mut self.pending_events,
            rename_state: &mut self.rename_state,
            cameras: &self.project.cameras,
        };

        let response = egui_dnd::dnd(ui, "uniform").show_custom(|ui, iter| {
            for (index, field) in uniform.fields().iter().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                ui.push_id(index, |ui| {
                    iter.next(ui, field.id(), index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            ui.horizontal(|ui| {
                                handle.ui(ui, |ui| {
                                    ui_uniform_field_title(ui, &mut ctx, uniform_id, index, field);
                                });
                                ui_uniform_type_label(ui, field.source().get_value().kind());
                            });

                            ui_uniform_field_entry(ui, &mut ctx, uniform_id, index, field);
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            self.pending_events
                .push(StateEvent::ReorderUniformField(uniform_id, update));
        }

        ui.add_space(6.0);

        ui.menu_button("Add Uniform", |ui| {
            for kind in UniformFieldSourceKind::iter() {
                if ui.button(kind.to_string()).clicked() {
                    let event = StateEvent::CreateUniformField(uniform_id, kind.into_source());
                    self.pending_events.push(event);
                }
            }
        });

        if !uniform.fields().is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click a");
                ui.label(RichText::new("Label").strong());
                ui.label("to remove it or drag it to reorder it.");
            }));
        }
    }
}

struct UniformUiContext<'a> {
    pending_events: &'a mut Vec<StateEvent>,
    rename_state: &'a mut Option<RenameState>,
    cameras: &'a Storage<CameraId, Camera>,
}

fn ui_uniform_field_title(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    uniform_id: UniformId,
    index: usize,
    field: &UniformField,
) {
    let rename_target = RenameTarget::UniformField(uniform_id, index);
    ui.add(renameable_label(
        Label::new(field.label())
            .selectable(false)
            .sense(Sense::click()),
        ctx.pending_events,
        ctx.rename_state,
        rename_target.clone(),
    ))
    .context_menu(|ui| {
        if ui.button("Delete Field").clicked() {
            ctx.pending_events
                .push(StateEvent::DeleteUniformField(uniform_id, index));
            ui.close();
        }
        if ui.button("Rename Field").clicked() {
            ctx.pending_events
                .push(StateEvent::StartRename(rename_target));
            ui.close();
        }
    });
}

fn ui_uniform_field_entry(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    uniform_id: UniformId,
    index: usize,
    field: &UniformField,
) {
    ui.indent("entry", |ui| {
        let event = egui::Grid::new("entry_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| ui_field_entry(ui, ctx, uniform_id, index, field))
            .inner;

        if let Some(event) = event {
            ctx.pending_events.push(event);
        }

        ui.collapsing("Current Values", |ui| {
            ui.horizontal(|ui| ui_uniform_field_data(ui, field.source().get_value()))
        });
    });
}

fn ui_field_entry(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    uniform_id: UniformId,
    index: usize,
    field: &UniformField,
) -> Option<StateEvent> {
    let mut source_kind = UniformFieldSourceKind::from_source(field.source());
    let source_kind_before = source_kind;
    ui.label("Source");
    egui::ComboBox::from_id_salt("source")
        .selected_text(source_kind.as_widget_text())
        .show_ui_list(ui, UniformFieldSourceKind::iter(), &mut source_kind);
    ui.end_row();

    let source_specific_event = match &field.source() {
        UniformFieldSource::UserDefined(data) => {
            let mut changed = false;
            let mut kind = data.kind();
            let kind_before = kind;
            ui.label("Type");
            egui::ComboBox::from_id_salt("type")
                .selected_text(kind.as_widget_text())
                .show_ui_list(ui, UniformFieldDataKind::iter(), &mut kind);
            ui.end_row();

            let mut data = data.clone();
            if kind_before != kind {
                data = UniformFieldData::from_kind(kind);
                changed = true;
            }

            ui.label("Data");
            ui.horizontal(|ui| {
                changed = edit_uniform_field_data(ui, &mut data);
            });
            ui.end_row();

            changed.then_some(StateEvent::UpdateUniformFieldSource(
                uniform_id,
                index,
                UniformFieldSource::UserDefined(data),
            ))
        }
        UniformFieldSource::Camera {
            camera_id, field, ..
        } => {
            let mut camera_id = *camera_id;
            let camera_id_before = camera_id;
            ui.label("Camera");
            egui::ComboBox::from_id_salt("camera")
                .selected_text_storage_opt(&ctx.cameras, camera_id)
                .show_ui_storage_opt(ui, &ctx.cameras, &mut camera_id);
            ui.end_row();
            ui.label("Field");
            let mut field = field.clone();
            let field_before = field.clone();
            egui::ComboBox::from_id_salt("camera_field")
                .selected_text(field.as_widget_text())
                .show_ui_list(ui, CameraField::iter(), &mut field);
            ui.end_row();

            (camera_id != camera_id_before || field != field_before).then_some(
                StateEvent::UpdateUniformFieldSource(
                    uniform_id,
                    index,
                    UniformFieldSource::new_camera_sourced(camera_id, field),
                ),
            )
        }
    };

    (source_kind != source_kind_before)
        .then_some(StateEvent::UpdateUniformFieldSource(
            uniform_id,
            index,
            source_kind.into_source(),
        ))
        .or(source_specific_event)
}

fn ui_uniform_type_label(ui: &mut Ui, kind: UniformFieldDataKind) {
    let (align, size) = kind.layout();
    egui::Popup::from_toggle_button_response(
        &ui.label(egui::RichText::new(kind.to_string()).weak())
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

fn edit_uniform_field_data(ui: &mut egui::Ui, data: &mut uniform::UniformFieldData) -> bool {
    let drag_value = |ui: &mut egui::Ui, value: &mut f32| {
        ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(2))
            .changed()
    };

    match data {
        uniform::UniformFieldData::Vec4f(vec4) => ui_array_mut(ui, vec4, drag_value),
        uniform::UniformFieldData::Vec3f(vec3) => ui_array_mut(ui, vec3, drag_value),
        uniform::UniformFieldData::Vec2f(vec2) => ui_array_mut(ui, vec2, drag_value),
        uniform::UniformFieldData::Mat4x4f(mat4) => {
            let mut changed = false;
            egui::Grid::new("fieldmat4").show(ui, |ui| {
                for row in mat4.iter_mut() {
                    changed |= ui_array_mut(ui, row, drag_value);
                    ui.end_row();
                }
            });
            changed
        }
        uniform::UniformFieldData::Rgba(color) => color_edit_rgba(ui, color).changed(),
        uniform::UniformFieldData::Rgb(color) => {
            egui::color_picker::color_edit_button_rgb(ui, color).changed()
        }
    }
}

fn ui_uniform_field_data(ui: &mut egui::Ui, data: &uniform::UniformFieldData) {
    let label = |ui: &mut egui::Ui, value: &f32| {
        ui.label(egui::RichText::new(format!("{value:.2}")).weak());
    };

    match data {
        uniform::UniformFieldData::Vec4f(vec4) => ui_array(ui, vec4, label),
        uniform::UniformFieldData::Vec3f(vec3) => ui_array(ui, vec3, label),
        uniform::UniformFieldData::Vec2f(vec2) => ui_array(ui, vec2, label),
        uniform::UniformFieldData::Mat4x4f(mat4) => {
            egui::Grid::new("fieldmat4").show(ui, |ui| {
                for row in mat4.iter() {
                    ui_array(ui, row, label);
                    ui.end_row();
                }
            });
        }
        uniform::UniformFieldData::Rgba(color) => ui_array(ui, color, label),
        uniform::UniformFieldData::Rgb(color) => ui_array(ui, color, label),
    }
}

impl AsWidgetText for UniformFieldSourceKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.to_string().into()
    }
}

impl AsWidgetText for UniformFieldDataKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.to_string().into()
    }
}

impl AsWidgetText for CameraField {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.to_string().into()
    }
}
