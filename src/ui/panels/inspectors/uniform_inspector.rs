use egui::{Label, Sense, Ui};
use strum::IntoEnumIterator;

use crate::{
    project::{
        UniformId,
        resource::{
            camera::Camera,
            uniform::{
                self, Transform, UniformField, UniformFieldData, UniformFieldDataKind,
                UniformFieldSource, UniformRuntimeField, camera::CameraField,
            },
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::{highlighted_label, shader_code_section},
            color_edit::color_edit_rgba,
            draggable_list::{ListEdits, draggable_list},
            field,
            field_docs::field_doc,
            inspector::{self, AsRichText},
            renameable_label::renameable_label,
            resource_icons,
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
    Transform,
    Time,
}

impl UniformFieldSourceKind {
    pub fn from_source(source: &UniformFieldSource) -> Self {
        match source {
            UniformFieldSource::UserDefined { .. } => Self::UserDefined,
            UniformFieldSource::Camera { .. } => Self::Camera,
            UniformFieldSource::Transform(..) => Self::Transform,
            UniformFieldSource::Time => Self::Time,
        }
    }

    pub fn into_source(self) -> UniformFieldSource {
        match self {
            Self::UserDefined => UniformFieldSource::new_user_defined(UniformFieldData::from_kind(
                UniformFieldDataKind::Vec3f,
            )),
            Self::Camera => UniformFieldSource::new_camera_sourced(None, CameraField::Position),
            Self::Transform => UniformFieldSource::new_transform(Transform::default()),
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

        inspector::section_doc(
            ui,
            "Layout",
            field_doc!(
                "How this uniform is packed into GPU memory. WGSL uniform buffers follow strict \
                alignment rules, so fields may be separated by **padding**, which wastes space.\n\n\
                Reorder fields (largest first) to reduce padding.\n\n\
                [WGSL spec](https://www.w3.org/TR/WGSL/#alignment-and-size)"
            ),
            |ui| {
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
                            field::error_label(ui, err.to_string());
                        }
                    }
                });
            },
        );

        let mut ctx = UniformUiContext {
            event_queue: self.event_queue,
            rename_state: self.rename_state,
            cameras: &self.project.cameras,
        };

        let mut fields = uniform.fields().to_vec();
        let mut rename_index: Option<usize> = None;
        const DEFAULT_NAME: &str = "Field";

        inspector::section_doc(
            ui,
            "Fields",
            field_doc!(
                "The ordered list of values packed into this uniform buffer and exposed to \
                shaders.\n\n\
                Each field has a **source** (a constant value, a Camera property, or time) and \
                is written to the GPU every frame. Drag to reorder, right-click to rename or \
                remove."
            ),
            |ui| {
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
                                ui_uniform_field_title(
                                    ui, &mut ctx, edits, uniform_id, index, field,
                                );
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

                ui.menu_button(resource_icons::add_text(ui, "Add Uniform"), |ui| {
                    for kind in UniformFieldSourceKind::iter() {
                        if ui.button(kind.to_string()).clicked() {
                            edits
                                .push_add_edit(UniformField::new(DEFAULT_NAME, kind.into_source()));
                            rename_index = Some(uniform.fields().len());
                        }
                    }
                });

                edits.apply(&mut fields);

                if fields != uniform.fields() {
                    uniform.set_fields(fields);
                }
            },
        );

        if let Some(index) = rename_index {
            *self.rename_state = Some(RenameState {
                target: RenameTarget::UniformField(uniform_id, index),
                current_label: DEFAULT_NAME.to_string(),
            });
        }

        if let Ok(uniform) = self.project.uniforms.get(uniform_id) {
            let ctx = ShaderGenCtx::from_project(self.project);
            shader_code_section(ui, uniform, &ctx);
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
    let label = resource_icons::drag_handle_text(ui, field.label());
    ui.add(renameable_label(
        Label::new(label).sense(Sense::click()),
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
        field::field_grid(ui, "entry_grid", |ui| {
            ui_field_entry(ui, ctx, index, field, edits)
        });

        ui.collapsing("Current Values", |ui| {
            if let Some(runtime_field) = runtime_field {
                ui_uniform_field_data(ui, runtime_field.data());
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
    let source_kind_changed = inspector::combo_row_doc(
        ui,
        "Source",
        field_doc!(
            "Where this field's value comes from each frame:\n\n\
            - **User Defined**: a constant value you set here.\n\
            - **Camera**: pulled from a Camera (position, matrices, and so on).\n\
            - **Transform**: a model matrix built from position, rotation, and scale.\n\
            - **Time**: the elapsed time in seconds, updated every frame."
        ),
        "source",
        UniformFieldSourceKind::iter(),
        &mut source_kind,
    );

    let source_specific_event = match &field.source() {
        UniformFieldSource::UserDefined(data) => {
            let mut changed = false;
            let mut kind = data.kind();
            let kind_changed = inspector::combo_row_doc(
                ui,
                "Type",
                field_doc!(
                    "The data type of this field (e.g. `f32`, `vec3<f32>`, `mat4x4<f32>`), \
                    which determines its size and alignment in the buffer.\n\n\
                    [WGSL spec](https://www.w3.org/TR/WGSL/#alignment-and-size)"
                ),
                "type",
                UniformFieldDataKind::iter(),
                &mut kind,
            );

            let mut data = data.clone();
            if kind_changed {
                data = UniformFieldData::from_kind(kind);
                changed = true;
            }

            field::row_doc(
                ui,
                "Data",
                field_doc!("The constant value stored in this field. Sent to the GPU as-is."),
                |ui| {
                    changed |= edit_uniform_field_data(ui, &mut data);
                },
            );

            changed.then_some(UniformFieldSource::UserDefined(data))
        }
        UniformFieldSource::Camera {
            camera_id, field, ..
        } => {
            let mut camera_id = *camera_id;
            let camera_id_before = camera_id;
            field::row_doc(
                ui,
                "Camera",
                field_doc!("The Camera this field reads its value from."),
                |ui| inspector::storage_combo(ui, "camera", ctx.cameras, &mut camera_id),
            );

            let mut field = *field;
            let field_before = field;
            inspector::combo_row_doc(
                ui,
                "Field",
                field_doc!(
                    "Which Camera property feeds this field, such as its position or one of \
                    its matrices."
                ),
                "camera_field",
                CameraField::iter(),
                &mut field,
            );

            (camera_id != camera_id_before || field != field_before)
                .then_some(UniformFieldSource::new_camera_sourced(camera_id, field))
        }
        UniformFieldSource::Transform(transform) => {
            let mut transform = *transform;
            let mut changed = false;

            let drag = |ui: &mut egui::Ui, array: &mut [f32; 3], speed: f32, suffix: &str| {
                let mut changed = false;
                ui.horizontal(|ui| {
                    for value in array.iter_mut() {
                        let drag_value = egui::DragValue::new(value)
                            .speed(speed)
                            .max_decimals(2)
                            .suffix(suffix);
                        changed |= ui.add(drag_value).changed();
                    }
                });
                changed
            };

            field::row_doc(
                ui,
                "Position",
                field_doc!("The translation applied to the model, in world units."),
                |ui| changed |= drag(ui, &mut transform.position, 0.01, ""),
            );
            field::row_doc(
                ui,
                "Rotation",
                field_doc!(
                    "Euler rotation in **degrees**, applied in XYZ order (X, then Y, then Z)."
                ),
                |ui| changed |= drag(ui, &mut transform.rotation, 0.1, "\u{00B0}"), // The degree symbol
            );
            field::row_doc(
                ui,
                "Scale",
                field_doc!(
                    "Per-axis scale factors. The composed matrix is `T * R * S` (scale first, \
                    then rotate, then translate)."
                ),
                |ui| changed |= drag(ui, &mut transform.scale, 0.01, ""),
            );

            changed.then_some(UniformFieldSource::new_transform(transform))
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
        &ui.label(egui::RichText::new(kind.to_string()).weak()),
    )
    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
    .show(|ui| {
        field::field_grid(ui, "uniform_type_layout", |ui| {
            field::row(ui, "Size", |ui| {
                ui.strong(format!("{size} bytes"));
            });
            field::row(ui, "Alignment", |ui| {
                ui.strong(format!("{align} bytes"));
            });
            field::row(ui, "Padding", |ui| {
                ui.strong(format!("{padding} bytes"));
            });
            field::row(ui, "WGSL type", |ui| {
                highlighted_label(ui, kind.wgsl_type_label(), "wgsl");
            });
        });
    });
}

fn edit_uniform_field_data(ui: &mut egui::Ui, data: &mut uniform::UniformFieldData) -> bool {
    let drag_float = |ui: &mut egui::Ui, value: &mut f32| {
        ui.add(egui::DragValue::new(value).speed(0.01).max_decimals(2))
            .changed()
    };

    let drag_int = |ui: &mut egui::Ui, value: &mut u32| {
        ui.add(egui::DragValue::new(value).speed(1.0).max_decimals(0))
            .changed()
    };

    let drag_float_array = |ui: &mut egui::Ui, array: &mut [f32]| {
        let mut changed = false;
        ui.horizontal(|ui| {
            for value in array.iter_mut() {
                changed |= drag_float(ui, value);
            }
        });
        changed
    };

    match data {
        uniform::UniformFieldData::UInt32(value) => drag_int(ui, value),
        uniform::UniformFieldData::Float(value) => drag_float(ui, value),
        uniform::UniformFieldData::Vec4f(vec4) => drag_float_array(ui, vec4),
        uniform::UniformFieldData::Vec3f(vec3) => drag_float_array(ui, vec3),
        uniform::UniformFieldData::Vec2f(vec2) => drag_float_array(ui, vec2),
        uniform::UniformFieldData::Mat4x4f(mat4) => {
            let mut changed = false;
            ui.vertical(|ui| {
                for row in mat4.iter_mut() {
                    changed |= drag_float_array(ui, row);
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
    let float_label = |ui: &mut egui::Ui, value: &f32| {
        ui.label(egui::RichText::new(format!("{value:.2}")).weak());
    };

    let int_label = |ui: &mut egui::Ui, value: &u32| {
        ui.label(egui::RichText::new(format!("{value}")).weak());
    };

    let array_float_label = |ui: &mut egui::Ui, array: &[f32]| {
        for value in array {
            float_label(ui, value);
        }
    };

    match data {
        uniform::UniformFieldData::UInt32(value) => int_label(ui, value),
        uniform::UniformFieldData::Float(value) => float_label(ui, value),
        uniform::UniformFieldData::Vec4f(vec4) => array_float_label(ui, vec4),
        uniform::UniformFieldData::Vec3f(vec3) => array_float_label(ui, vec3),
        uniform::UniformFieldData::Vec2f(vec2) => array_float_label(ui, vec2),
        uniform::UniformFieldData::Mat4x4f(mat4) => {
            egui::Grid::new("fieldmat4").show(ui, |ui| {
                for row in mat4.iter() {
                    array_float_label(ui, row);
                    ui.end_row();
                }
            });
        }
        uniform::UniformFieldData::Rgba(color) => array_float_label(ui, color),
        uniform::UniformFieldData::Rgb(color) => array_float_label(ui, color),
    }
}

impl AsRichText for UniformFieldSourceKind {
    fn as_rich_text(&self) -> egui::RichText {
        self.to_string().into()
    }
}

impl AsRichText for UniformFieldDataKind {
    fn as_rich_text(&self) -> egui::RichText {
        self.to_string().into()
    }
}

impl AsRichText for CameraField {
    fn as_rich_text(&self) -> egui::RichText {
        self.to_string().into()
    }
}
