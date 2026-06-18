use egui::{CollapsingHeader, RichText};
use glam::Vec3;

use crate::{
    project::{
        CameraId,
        resource::camera::{CameraMode, Deg, Fov, LookAt, Pitch, PositiveF32, Yaw},
    },
    ui::{
        components::{
            data_display::ui_mat4_grid,
            field_docs::{FieldDoc, field_doc},
            inspector,
        },
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn camera_inspector_ui(&mut self, ui: &mut egui::Ui, camera_id: CameraId) {
        let Ok(camera) = self.project.cameras.get_mut(camera_id) else {
            ui.label("Camera couldn't be found.");
            return;
        };

        let camera_runtime = self.runtime_project.cameras.get_init(camera_id);

        inspector::section(ui, "Transform", |ui| {
            inspector::field_grid(ui, "camera_transform_grid", |ui| {
                let mut mode = camera.mode();
                let mode_changed = inspector::row_doc(
                    ui,
                    "Mode",
                    field_doc!(
                        "Controls how the camera moves and orients.\n\n\
                        - **First Person**: the camera rotates in place around its own \
                        position (free-look / fly camera).\n\
                        - **Third Person**: the camera orbits around a target point (the \
                        **Looking At** position), keeping it framed."
                    ),
                    |ui| {
                        ui.horizontal(|ui| {
                            let first = ui
                                .radio_value(&mut mode, CameraMode::FirstPerson, "First Person")
                                .changed();
                            let third = ui
                                .radio_value(&mut mode, CameraMode::ThirdPerson, "Third Person")
                                .changed();
                            first || third
                        })
                        .inner
                    },
                );
                if mode_changed {
                    camera.set_mode(mode);
                }

                let ui_position_drag = |ui: &mut egui::Ui, axis: &mut f32| {
                    let drag_value = egui::DragValue::new(axis).speed(0.01).max_decimals(3);
                    ui.add(drag_value).changed()
                };

                if camera.mode() == CameraMode::ThirdPerson {
                    inspector::row_doc(
                        ui,
                        "Looking At",
                        field_doc!(
                            "World-space point the camera orbits and faces. Only available in \
                            **Third Person** mode. Editing it re-aims the camera at the new \
                            target from its current position."
                        ),
                        |ui| {
                            ui.horizontal(|ui| {
                                let target = camera.looking_at();
                                let (mut x, mut y, mut z) = (target.x, target.y, target.z);
                                let cx = ui_position_drag(ui, &mut x);
                                let cy = ui_position_drag(ui, &mut y);
                                let cz = ui_position_drag(ui, &mut z);

                                if cx || cy || cz {
                                    let target = Vec3::new(x, y, z);
                                    camera.set_looking_at(LookAt::new(camera.position(), target));
                                }
                            });
                        },
                    );
                }

                inspector::row_doc(
                    ui,
                    "Position",
                    field_doc!("World-space `(x, y, z)` location of the camera (the eye point)."),
                    |ui| {
                        ui.horizontal(|ui| {
                            let pos = camera.position();
                            let (mut x, mut y, mut z) = (pos.x, pos.y, pos.z);
                            let cx = ui_position_drag(ui, &mut x);
                            let cy = ui_position_drag(ui, &mut y);
                            let cz = ui_position_drag(ui, &mut z);

                            if cx || cy || cz {
                                camera.set_position(Vec3::new(x, y, z));
                            }
                        });
                    },
                );

                let Deg(mut yaw) = (*camera.yaw()).into();
                if degree_drag_row(
                    ui,
                    "Yaw",
                    field_doc!(
                        "Horizontal rotation (turning left and right) around the vertical \
                        axis, in degrees."
                    ),
                    &mut yaw,
                    f32::MIN..=f32::MAX,
                ) {
                    camera.set_yaw(Yaw::new(Deg(yaw)));
                }

                let Deg(mut pitch) = (*camera.pitch()).into();
                if degree_drag_row(
                    ui,
                    "Pitch",
                    field_doc!(
                        "Vertical rotation (looking up and down) around the horizontal \
                        axis, in degrees."
                    ),
                    &mut pitch,
                    -89.9_f32..=89.9_f32,
                ) {
                    camera.set_pitch(Pitch::new(Deg(pitch)));
                }
            });
        });

        inspector::section(ui, "Projection", |ui| {
            inspector::field_grid(ui, "camera_projection_grid", |ui| {
                let Deg(mut fov) = (*camera.fovy()).into();
                if degree_drag_row(
                    ui,
                    "FOV",
                    field_doc!(
                        "Vertical **field of view**: the angle the camera sees from the bottom \
                        to the top of the frame, in degrees."
                    ),
                    &mut fov,
                    1.0..=179.0,
                ) {
                    camera.set_fovy(Fov::new(Deg(fov)));
                }

                let mut znear = camera.clip().znear();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Near Clip",
                    field_doc!(
                        "Distance to the **near clipping plane**. Anything closer to the camera \
                        than this is not drawn.\n\n\
                        Must be greater than `0`; very small values reduce depth-buffer precision."
                    ),
                    &mut znear,
                    0.0001..=f32::MAX,
                    0.001,
                    4,
                ) {
                    camera.set_clip(camera.clip().with_znear(znear));
                }

                let mut zfar = camera.clip().zfar();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Far Clip",
                    field_doc!(
                        "Distance to the **far clipping plane**. Anything farther from the \
                        camera than this is not drawn.\n\n\
                        The far-to-near ratio sets depth precision, so avoid making this \
                        needlessly large."
                    ),
                    &mut zfar,
                    0.0001..=f32::MAX,
                    0.001,
                    4,
                ) {
                    camera.set_clip(camera.clip().with_zfar(zfar));
                }

                inspector::row_doc(
                    ui,
                    "Aspect",
                    field_doc!(
                        "Width / height of the render target.\n\n\
                        Read-only. Derived from the selected **Dimension**."
                    ),
                    |ui| match &camera_runtime {
                        Ok(Some(camera_runtime)) => ui
                            .label(RichText::new(format!("{:.4}", camera_runtime.aspect())).weak()),
                        Ok(None) => ui.spinner(),
                        Err(err) => inspector::error_label(ui, err.to_string()),
                    },
                );

                let mut current_dim_id = camera.dimension_id();
                if inspector::row_doc(
                    ui,
                    "Dimension",
                    field_doc!(
                        "The Dimension that sets the camera's **aspect ratio**.\n\n\
                        Use the **same Dimension as the viewport** the camera renders to, \
                        otherwise the image looks stretched or squashed. The projection matrix \
                        updates automatically when the Dimension resizes."
                    ),
                    |ui| {
                        inspector::storage_combo(
                            ui,
                            "camera_aspect_source",
                            &self.project.dimensions,
                            &mut current_dim_id,
                        )
                    },
                ) {
                    camera.set_dimension_id(current_dim_id);
                }
            });
        });

        inspector::section(ui, "Movement Parameters", |ui| {
            inspector::field_grid(ui, "camera_movement_grid", |ui| {
                inspector::row_doc(
                    ui,
                    "Speed",
                    field_doc!(
                        "Current per-axis movement velocity, in units per second.\n\n\
                        Read-only. Reflects live movement."
                    ),
                    |ui| {
                        let s = camera.current_speed();
                        ui.label(RichText::new(format!("{:.3}", s.x)).weak());
                        ui.label(RichText::new(format!("{:.3}", s.y)).weak());
                        ui.label(RichText::new(format!("{:.3}", s.z)).weak());
                    },
                );

                inspector::row_doc(
                    ui,
                    "Scroll Speed",
                    field_doc!(
                        "Current zoom velocity from scrolling, in units per second.\n\n\
                        Read-only. Reflects live movement."
                    ),
                    |ui| {
                        let s = camera.current_scroll_speed();
                        ui.label(RichText::new(format!("{s:.3}")).weak());
                    },
                );

                let mut max_speed = *camera.max_speed();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Max Speed",
                    field_doc!("Upper limit on movement speed, in units per second."),
                    &mut max_speed,
                    0.0..=f32::MAX,
                    0.1,
                    2,
                ) {
                    camera.set_max_speed(PositiveF32::new(max_speed));
                }

                let mut acceleration = *camera.acceleration();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Acceleration",
                    field_doc!(
                        "How quickly the camera ramps up toward **Max Speed** while a movement \
                        key is held, in units per second squared."
                    ),
                    &mut acceleration,
                    0.0..=f32::MAX,
                    0.1,
                    2,
                ) {
                    camera.set_acceleration(PositiveF32::new(acceleration));
                }

                let mut drag = *camera.drag();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Drag",
                    field_doc!(
                        "How quickly the camera coasts to a stop once movement input stops. \
                        Higher values bring it to rest faster."
                    ),
                    &mut drag,
                    0.0..=f32::MAX,
                    0.01,
                    2,
                ) {
                    camera.set_drag_factor(PositiveF32::new(drag));
                }

                let mut sensitivity = *camera.sensitivity();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Sensitivity",
                    field_doc!("How much the camera rotates per unit of mouse movement."),
                    &mut sensitivity,
                    0.0..=f32::MAX,
                    0.001,
                    4,
                ) {
                    camera.set_sensitivity(PositiveF32::new(sensitivity));
                }

                let mut scroll_sensitivity = *camera.scroll_sensitivity();
                if inspector::f32_drag_row_doc(
                    ui,
                    "Scroll Sensitivity",
                    field_doc!("How much the camera zooms per unit of scrolling."),
                    &mut scroll_sensitivity,
                    0.0..=f32::MAX,
                    0.01,
                    4,
                ) {
                    camera.set_scroll_sensitivity(PositiveF32::new(scroll_sensitivity));
                }
            });
        });

        inspector::section(ui, "Matrices", |ui| {
            let matrix = match &camera_runtime {
                Ok(Some(camera_runtime)) => camera_runtime.matrix(),
                Ok(None) => {
                    ui.spinner();
                    return;
                }
                Err(err) => {
                    inspector::error_label(ui, err.to_string());
                    return;
                }
            };

            CollapsingHeader::new("Projection")
                .id_salt("mat_projection")
                .default_open(false)
                .show(ui, |ui| {
                    ui_mat4_grid(ui, &matrix.projection.to_cols_array_2d());
                });

            CollapsingHeader::new("View")
                .id_salt("mat_view")
                .default_open(false)
                .show(ui, |ui| {
                    ui_mat4_grid(ui, &matrix.view.to_cols_array_2d());
                });

            CollapsingHeader::new("Projection View")
                .id_salt("mat_projection_view")
                .default_open(false)
                .show(ui, |ui| {
                    ui_mat4_grid(ui, &matrix.projection_view.to_cols_array_2d());
                });

            CollapsingHeader::new("Inverse Projection")
                .id_salt("mat_inv_projection")
                .default_open(false)
                .show(ui, |ui| {
                    ui_mat4_grid(ui, &matrix.inv_proj.to_cols_array_2d());
                });

            CollapsingHeader::new("Inverse View")
                .id_salt("mat_inv_view")
                .default_open(false)
                .show(ui, |ui| {
                    ui_mat4_grid(ui, &matrix.inverse_view.to_cols_array_2d());
                });
        });
    }
}

fn degree_drag_row(
    ui: &mut egui::Ui,
    label: &'static str,
    doc: impl FieldDoc,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    inspector::row_doc(ui, label, doc, |ui| {
        ui.add(
            egui::DragValue::new(value)
                .speed(0.5)
                .max_decimals(2)
                .suffix("\u{00B0}") // The degree symbol
                .range(range),
        )
        .changed()
    })
}
