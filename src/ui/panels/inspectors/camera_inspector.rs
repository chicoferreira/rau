use cgmath::{Deg, Point3};
use egui::{CollapsingHeader, Grid, RichText};

use crate::{
    project::CameraId,
    ui::{
        components::{data_display::ui_mat4_grid, selector::selectable_value},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn camera_inspector_ui(&mut self, ui: &mut egui::Ui, camera_id: CameraId) {
        let Some(camera) = self.project.cameras.get_mut(camera_id) else {
            ui.label("Camera couldn't be found.");
            return;
        };

        CollapsingHeader::new("Transform")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("camera_transform_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Position");
                        ui.horizontal(|ui| {
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
                                camera.set_position(Point3::new(x, y, z));
                            }
                        });
                        ui.end_row();

                        ui.label("Yaw");
                        let Deg(mut yaw) = camera.yaw().into();
                        let widget = egui::DragValue::new(&mut yaw)
                            .speed(0.5)
                            .max_decimals(2)
                            .suffix("°");
                        if ui.add(widget).changed() {
                            camera.set_yaw(Deg(yaw));
                        }
                        ui.end_row();

                        ui.label("Pitch");
                        let Deg(mut pitch) = camera.pitch().into();
                        let widget = egui::DragValue::new(&mut pitch)
                            .speed(0.5)
                            .max_decimals(2)
                            .suffix("°")
                            .range(-89.9_f32..=89.9_f32);
                        if ui.add(widget).changed() {
                            camera.set_pitch(Deg(pitch));
                        }
                        ui.end_row();
                    });
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Projection")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("camera_projection_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("FOV");
                        let Deg(mut fov) = camera.fovy().into();
                        if ui
                            .add(egui::Slider::new(&mut fov, 1.0_f32..=179.0_f32).suffix("°"))
                            .changed()
                        {
                            camera.set_fovy(Deg(fov));
                        }
                        ui.end_row();

                        ui.label("Near Clip");
                        let mut znear = camera.znear();
                        let widget = egui::DragValue::new(&mut znear)
                            .speed(0.001)
                            .max_decimals(4)
                            .range(0.0001_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_znear(znear);
                        }
                        ui.end_row();

                        ui.label("Far Clip");
                        let mut zfar = camera.zfar();
                        let widget = egui::DragValue::new(&mut zfar)
                            .speed(0.1)
                            .max_decimals(2)
                            .range(0.001_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_zfar(zfar);
                        }
                        ui.end_row();

                        ui.label("Aspect");

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{:.4}", camera.aspect())).weak());
                            ui.label("from");
                            let mut current_dim_id = camera.dimension_id();
                            let before = current_dim_id;
                            selectable_value(
                                ui,
                                "camera_aspect_source",
                                &mut current_dim_id,
                                |_id, dim| dim.label.as_str(),
                                &self.project.dimensions,
                            );
                            if before != current_dim_id {
                                camera.set_dimension_id(current_dim_id);
                            }
                        });

                        ui.end_row();
                    });
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Movement Parameters")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("camera_movement_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Speed");
                        let s = camera.current_speed();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{:.3}", s.x)).weak());
                            ui.label(RichText::new(format!("{:.3}", s.y)).weak());
                            ui.label(RichText::new(format!("{:.3}", s.z)).weak());
                        });
                        ui.end_row();

                        ui.label("Max Speed");
                        let mut v = camera.max_speed();
                        let widget = egui::DragValue::new(&mut v)
                            .speed(0.1)
                            .max_decimals(2)
                            .range(0.0_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_max_speed(v);
                        }
                        ui.end_row();

                        ui.label("Acceleration");
                        let mut v = camera.acceleration();
                        let widget = egui::DragValue::new(&mut v)
                            .speed(0.1)
                            .max_decimals(2)
                            .range(0.0_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_acceleration(v);
                        }
                        ui.end_row();

                        ui.label("Drag");
                        let mut v = camera.drag_factor();
                        let widget = egui::DragValue::new(&mut v)
                            .speed(0.01)
                            .max_decimals(2)
                            .range(0.0_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_drag_factor(v);
                        }
                        ui.end_row();

                        ui.label("Sensitivity");
                        let mut v = camera.sensitivity();
                        let widget = egui::DragValue::new(&mut v)
                            .speed(0.001)
                            .max_decimals(4)
                            .range(0.0_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_sensitivity(v);
                        }
                        ui.end_row();

                        ui.label("Scroll Speed");
                        let mut v = camera.scroll_speed();
                        let widget = egui::DragValue::new(&mut v)
                            .speed(0.01)
                            .max_decimals(4)
                            .range(0.0_f32..=f32::MAX);
                        if ui.add(widget).changed() {
                            camera.set_scroll_speed(v);
                        }
                        ui.end_row();
                    });
            });

        ui.add_space(4.0);

        let matrix = *camera.matrix();
        CollapsingHeader::new("Matrices")
            .default_open(false)
            .show(ui, |ui| {
                CollapsingHeader::new("Projection")
                    .id_salt("mat_projection")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui_mat4_grid(ui, &matrix.projection.into());
                    });

                CollapsingHeader::new("View")
                    .id_salt("mat_view")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui_mat4_grid(ui, &matrix.view.into());
                    });

                CollapsingHeader::new("Projection View")
                    .id_salt("mat_projection_view")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui_mat4_grid(ui, &matrix.projection_view.into());
                    });

                CollapsingHeader::new("Inverse Projection")
                    .id_salt("mat_inv_projection")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui_mat4_grid(ui, &matrix.inverse_projection.into());
                    });

                CollapsingHeader::new("Inverse View")
                    .id_salt("mat_inv_view")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui_mat4_grid(ui, &matrix.inverse_view.into());
                    });
            });
    }
}
