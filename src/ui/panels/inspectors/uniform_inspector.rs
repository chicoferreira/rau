use egui::{Label, RichText, Sense, Ui};
use strum::IntoEnumIterator;

use crate::{
    project::{
        UniformId,
        resource::{
            camera::Camera,
            uniform::{
                self, UniformField, UniformFieldData, UniformFieldDataKind, UniformFieldSource,
                UniformRuntimeField, camera::CameraField,
            },
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::{highlighted_label, shader_code_section},
            color_edit::color_edit_rgba,
            data_display::{ui_array, ui_array_mut},
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector,
            renameable_label::renameable_label,
            selector::AsWidgetText,
        },
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
    },
    utils::{event_queue::EventQueue, shader_preview::ShaderGenCtx},
    workspace::StateEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
pub enum UniformFieldSourceKind {
    #[strum(to_string = "User Defined")]
    UserDefined,
    Camera,
    Time,
}

impl UniformFieldSourceKind {
    pub fn from_source(source: &UniformFieldSource) -> Self {
        match source {
            UniformFieldSource::UserDefined { .. } => Self::UserDefined,
            UniformFieldSource::Camera { .. } => Self::Camera,
            UniformFieldSource::Time => Self::Time,
        }
    }

    pub fn into_source(self) -> UniformFieldSource {
        match self {
            Self::UserDefined => UniformFieldSource::new_user_defined(UniformFieldData::from_kind(
                UniformFieldDataKind::Vec3f,
            )),
            Self::Camera => UniformFieldSource::new_camera_sourced(None, CameraField::Position),
            Self::Time => UniformFieldSource::new_time(),
        }
    }
}

impl StateSnapshot<'_> {
    pub fn uniform_inspector_ui(&mut self, uniform_id: UniformId, ui: &mut egui::Ui) {
        let Ok(uniform) = self.project.uniforms.get_mut(uniform_id) else {
            ui.label("Uniform couldn't be found.");
            return;
        };

        let uniform_runtime = self.runtime_project.uniforms.get_init(uniform_id);
        let uniform_layout = match &uniform_runtime {
            Ok(Some(uniform_runtime)) => Ok(Some(uniform_runtime.layout())),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        };

        ui.horizontal(|ui| {
            ui.label("Total size");
            match &uniform_layout {
                Ok(Some(uniform_layout)) => {
                    ui.strong(format!("{} bytes", uniform_layout.size));

                    let padding = uniform_layout.padding;
                    if padding > 0 {
                        ui.weak(format!("({padding} bytes wasted on padding)"));
                    }
                }
                Ok(None) => {
                    ui.spinner();
                }
                Err(err) => {
                    ui.colored_label(ui.visuals().error_fg_color, err.to_string());
                }
            }
        });

        ui.add_space(6.0);

        let mut ctx = UniformUiContext {
            event_queue: &mut self.event_queue,
            rename_state: &mut self.rename_state,
            cameras: &self.project.cameras,
        };

        let mut fields = uniform.fields().to_vec();

        let mut edits = draggable_list(
            ui,
            ("uniform", uniform_id),
            &fields,
            |ui, field, index, handle, edits| {
                let runtime_field = match &uniform_runtime {
                    Ok(Some(uniform_runtime)) => uniform_runtime.fields().get(index),
                    Ok(None) | Err(_) => None,
                };

                ui.horizontal(|ui| {
                    handle.ui(ui, |ui| {
                        ui_uniform_field_title(ui, &mut ctx, edits, uniform_id, index, field);
                    });
                    if let Some(runtime_field) = runtime_field {
                        let padding = match &uniform_layout {
                            Ok(Some(uniform_layout)) => {
                                let padding = uniform_layout.field_paddings.get(index);
                                padding.copied().unwrap_or(0)
                            }
                            Ok(None) | Err(_) => 0,
                        };
                        ui_uniform_type_label(ui, runtime_field.data().kind(), padding);
                    }
                });

                ui_uniform_field_entry(ui, &mut ctx, edits, index, field, runtime_field);
            },
        );

        ui.add_space(6.0);

        let mut rename_index: Option<usize> = None;

        const DEFAULT_NAME: &str = "Field";

        ui.menu_button("Add Uniform", |ui| {
            for kind in UniformFieldSourceKind::iter() {
                if ui.button(kind.to_string()).clicked() {
                    edits.push_add_edit(UniformField::new(DEFAULT_NAME, kind.into_source()));
                    rename_index = Some(uniform.fields().len());
                }
            }
        });

        if let Some(index) = rename_index {
            *self.rename_state = Some(RenameState {
                target: RenameTarget::UniformField(uniform_id, index),
                current_label: DEFAULT_NAME.to_string(),
            });
        }

        edits.apply(&mut fields);

        if &fields != uniform.fields() {
            uniform.set_fields(fields);
        }

        if !uniform.fields().is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click a");
                ui.label(RichText::new("Label").strong());
                ui.label("to remove it or drag it to reorder it.");
            }));
        }

        if let Ok(uniform) = self.project.uniforms.get(uniform_id) {
            let ctx = ShaderGenCtx::from_project(self.project);
            shader_code_section(ui, (uniform_id, "shader_code"), uniform, &ctx);
        }
    }
}

struct UniformUiContext<'a> {
    event_queue: &'a mut EventQueue<StateEvent>,
    rename_state: &'a mut Option<RenameState>,
    cameras: &'a Storage<Camera>,
}

fn ui_uniform_field_title(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    edits: &mut ListEdits<UniformField>,
    uniform_id: UniformId,
    index: usize,
    field: &UniformField,
) {
    let rename_target = RenameTarget::UniformField(uniform_id, index);
    ui.add(renameable_label(
        Label::new(field.label())
            .selectable(false)
            .sense(Sense::click()),
        ctx.event_queue,
        ctx.rename_state,
        rename_target.clone(),
    ))
    .context_menu(|ui| {
        if ui.button("Rename Field").clicked() {
            ctx.event_queue.start_rename(rename_target);
            ui.close();
        }
        if ui.button("Delete Field").clicked() {
            edits.push_remove_edit(index);
            ui.close();
        }
    });
}

fn ui_uniform_field_entry(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    edits: &mut ListEdits<UniformField>,
    index: usize,
    field: &UniformField,
    runtime_field: Option<&UniformRuntimeField>,
) {
    ui.indent("entry", |ui| {
        inspector::field_grid(ui, "entry_grid", |ui| {
            ui_field_entry(ui, ctx, index, field, edits)
        });

        ui.collapsing("Current Values", |ui| {
            if let Some(runtime_field) = runtime_field {
                ui.horizontal(|ui| ui_uniform_field_data(ui, runtime_field.data()));
            } else {
                ui.weak("Runtime values are not available.");
            }
        });
    });
}

fn ui_field_entry(
    ui: &mut Ui,
    ctx: &mut UniformUiContext,
    index: usize,
    field: &UniformField,
    edits: &mut ListEdits<UniformField>,
) {
    let mut source_kind = UniformFieldSourceKind::from_source(field.source());
    let source_kind_changed = inspector::combo_row(
        ui,
        "Source",
        "source",
        UniformFieldSourceKind::iter(),
        &mut source_kind,
    );

    let source_specific_event = match &field.source() {
        UniformFieldSource::UserDefined(data) => {
            let mut changed = false;
            let mut kind = data.kind();
            let kind_changed =
                inspector::combo_row(ui, "Type", "type", UniformFieldDataKind::iter(), &mut kind);

            let mut data = data.clone();
            if kind_changed {
                data = UniformFieldData::from_kind(kind);
                changed = true;
            }

            inspector::row(ui, "Data", |ui| {
                changed |= edit_uniform_field_data(ui, &mut data);
            });

            changed.then_some(UniformFieldSource::UserDefined(data))
        }
        UniformFieldSource::Camera {
            camera_id, field, ..
        } => {
            let mut camera_id = *camera_id;
            let camera_id_before = camera_id;
            inspector::storage_opt_combo_row(ui, "Camera", "camera", &ctx.cameras, &mut camera_id);

            let mut field = field.clone();
            let field_before = field.clone();
            inspector::combo_row(ui, "Field", "camera_field", CameraField::iter(), &mut field);

            (camera_id != camera_id_before || field != field_before)
                .then_some(UniformFieldSource::new_camera_sourced(camera_id, field))
        }
        UniformFieldSource::Time => None,
    };

    if let Some(new_source) = source_kind_changed
        .then_some(source_kind.into_source())
        .or(source_specific_event)
    {
        let field = UniformField::new(field.label(), new_source);
        edits.push_set_edit(index, field);
    }
}

fn ui_uniform_type_label(ui: &mut Ui, kind: UniformFieldDataKind, padding: usize) {
    let (align, size) = kind.layout();
    egui::Popup::from_toggle_button_response(
        &ui.label(egui::RichText::new(kind.to_string()).weak())
            .on_hover_cursor(egui::CursorIcon::PointingHand),
    )
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .show(|ui| {
        inspector::field_grid(ui, "uniform_type_layout", |ui| {
            inspector::row(ui, "Size", |ui| {
                ui.strong(format!("{size} bytes"));
            });
            inspector::row(ui, "Alignment", |ui| {
                ui.strong(format!("{align} bytes"));
            });
            inspector::row(ui, "Padding", |ui| {
                ui.strong(format!("{padding} bytes"));
            });
            inspector::row(ui, "WGSL type", |ui| {
                highlighted_label(ui, kind.wgsl_type_label(), "wgsl");
            });
        });
    });
}

fn edit_uniform_field_data(ui: &mut egui::Ui, data: &mut uniform::UniformFieldData) -> bool {
    let drag_value = |ui: &mut egui::Ui, value: &mut f32| {
        ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(2))
            .changed()
    };

    ui.horizontal(|ui| match data {
        uniform::UniformFieldData::Float(value) => drag_value(ui, value),
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
    })
    .inner
}

fn ui_uniform_field_data(ui: &mut egui::Ui, data: &uniform::UniformFieldData) {
    let label = |ui: &mut egui::Ui, value: &f32| {
        ui.label(egui::RichText::new(format!("{value:.2}")).weak());
    };

    match data {
        uniform::UniformFieldData::Float(value) => label(ui, value),
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
