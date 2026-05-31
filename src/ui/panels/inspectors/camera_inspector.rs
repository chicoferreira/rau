use egui::{CollapsingHeader, RichText};
use glam::Vec3;

use crate::{
    project::{
        CameraId,
        resource::camera::{Deg, Fov, Pitch, PositiveF32, Yaw},
    },
    ui::{
        components::{data_display::ui_mat4_grid, inspector},
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

        CollapsingHeader::new("Transform")
            .default_open(true)
            .show(ui, |ui| {
                inspector::field_grid(ui, "camera_transform_grid", |ui| {
                    inspector::row(ui, "Position", |ui| {
                        let pos = camera.position();
                        let (mut x, mut y, mut z) = (pos.x, pos.y, pos.z);
                        let cx = ui
                            .add(egui::DragValue::new(&mut x).speed(0.01).max_decimals(3))
                            .changed();
                        let cy = ui
                            .add(egui::DragValue::new(&mut y).speed(0.01).max_decimals(3))
                            .changed();
                        let cz = ui
                            .add(egui::DragValue::new(&mut z).speed(0.01).max_decimals(3))
                            .changed();
                        if cx || cy || cz {
                            camera.set_position(Vec3::new(x, y, z));
                        }
                    });

                    let Deg(mut yaw) = (*camera.yaw()).into();
                    if degree_drag_row(ui, "Yaw", &mut yaw, f32::MIN..=f32::MAX) {
                        camera.set_yaw(Yaw::new(Deg(yaw)));
                    }

                    let Deg(mut pitch) = (*camera.pitch()).into();
                    if degree_drag_row(ui, "Pitch", &mut pitch, -89.9_f32..=89.9_f32) {
                        camera.set_pitch(Pitch::new(Deg(pitch)));
                    }
                });
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Projection")
            .default_open(true)
            .show(ui, |ui| {
                inspector::field_grid(ui, "camera_projection_grid", |ui| {
                    let Deg(mut fov) = (*camera.fovy()).into();
                    if degree_drag_row(ui, "FOV", &mut fov, 1.0..=179.0) {
                        camera.set_fovy(Fov::new(Deg(fov)));
                    }

                    let mut znear = camera.clip().znear();
                    if inspector::f32_drag_row(
                        ui,
                        "Near Clip",
                        &mut znear,
                        0.0001..=f32::MAX,
                        0.001,
                        4,
                    ) {
                        camera.set_clip(camera.clip().with_znear(znear));
                    }

                    let mut zfar = camera.clip().zfar();
                    if inspector::f32_drag_row(
                        ui,
                        "Far Clip",
                        &mut zfar,
                        0.0001..=f32::MAX,
                        0.001,
                        4,
                    ) {
                        camera.set_clip(camera.clip().with_zfar(zfar));
                    }

                    inspector::row(ui, "Aspect", |ui| match &camera_runtime {
                        Ok(Some(camera_runtime)) => ui
                            .label(RichText::new(format!("{:.4}", camera_runtime.aspect())).weak()),
                        Ok(None) => ui.spinner(),
                        Err(err) => ui.colored_label(ui.visuals().error_fg_color, err.to_string()),
                    });

                    let mut current_dim_id = camera.dimension_id();
                    if inspector::storage_opt_combo_row(
                        ui,
                        "Dimension",
                        "camera_aspect_source",
                        &self.project.dimensions,
                        &mut current_dim_id,
                    ) {
                        camera.set_dimension_id(current_dim_id);
                    }
                });
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Movement Parameters")
            .default_open(true)
            .show(ui, |ui| {
                inspector::field_grid(ui, "camera_movement_grid", |ui| {
                    inspector::row(ui, "Speed", |ui| {
                        let s = camera.current_speed();
                        ui.label(RichText::new(format!("{:.3}", s.x)).weak());
                        ui.label(RichText::new(format!("{:.3}", s.y)).weak());
                        ui.label(RichText::new(format!("{:.3}", s.z)).weak());
                    });

                    let mut max_speed = *camera.max_speed();
                    if inspector::f32_drag_row(
                        ui,
                        "Max Speed",
                        &mut max_speed,
                        0.0..=f32::MAX,
                        0.1,
                        2,
                    ) {
                        camera.set_max_speed(PositiveF32::new(max_speed));
                    }

                    let mut acceleration = *camera.acceleration();
                    if inspector::f32_drag_row(
                        ui,
                        "Acceleration",
                        &mut acceleration,
                        0.0..=f32::MAX,
                        0.1,
                        2,
                    ) {
                        camera.set_acceleration(PositiveF32::new(acceleration));
                    }

                    let mut drag = *camera.drag();
                    if inspector::f32_drag_row(ui, "Drag", &mut drag, 0.0..=f32::MAX, 0.01, 2) {
                        camera.set_drag_factor(PositiveF32::new(drag));
                    }

                    let mut sensitivity = *camera.sensitivity();
                    if inspector::f32_drag_row(
                        ui,
                        "Sensitivity",
                        &mut sensitivity,
                        0.0..=f32::MAX,
                        0.001,
                        4,
                    ) {
                        camera.set_sensitivity(PositiveF32::new(sensitivity));
                    }

                    let mut scroll_speed = *camera.scroll_speed();
                    if inspector::f32_drag_row(
                        ui,
                        "Scroll Speed",
                        &mut scroll_speed,
                        0.0..=f32::MAX,
                        0.01,
                        4,
                    ) {
                        camera.set_scroll_speed(PositiveF32::new(scroll_speed));
                    }
                });
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Matrices")
            .default_open(false)
            .show(ui, |ui| {
                let matrix = match &camera_runtime {
                    Ok(Some(camera_runtime)) => camera_runtime.matrix(),
                    Ok(None) => {
                        ui.spinner();
                        return;
                    }
                    Err(err) => {
                        ui.colored_label(ui.visuals().error_fg_color, err.to_string());
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
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    inspector::row(ui, label, |ui| {
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
