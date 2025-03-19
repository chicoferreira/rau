use crate::renderer::uniform::{
    BindingResourceType, CustomUniform, TimeUniformData, UniformResourceType,
};
use crate::renderer::Renderer;
use egui::color_picker::Alpha;
use egui::text::LayoutJob;
use egui::{RichText, Style};
use enum2egui::GuiInspect;

pub fn render_gui(renderer: &mut Renderer) {
    let context = &mut renderer.egui.context;
    egui::Window::new("Rau")
        .default_open(true)
        .show(context, |ui| {
            ui.heading("Viewport");
            ui.horizontal(|ui| {
                ui.label("Clear Color");
                let color = renderer.renderer_project.viewport_clear_color;
                let mut rgba = egui::Rgba::from_rgba_premultiplied(
                    color.r as f32,
                    color.g as f32,
                    color.b as f32,
                    color.a as f32,
                );
                if egui::color_picker::color_edit_button_rgba(ui, &mut rgba, Alpha::OnlyBlend)
                    .changed()
                {
                    renderer.renderer_project.viewport_clear_color = wgpu::Color {
                        r: rgba[0] as f64,
                        g: rgba[1] as f64,
                        b: rgba[2] as f64,
                        a: rgba[3] as f64,
                    };
                };
            });
            ui.heading("Camera");
            renderer.renderer_project.camera.ui_mut(ui);

            ui.heading("Textures");
            for (index, texture) in renderer.renderer_project.textures.iter().enumerate() {
                ui.collapsing(RichText::from(&texture.name).strong(), |ui| {
                    if let Some(&texture_id) = renderer.renderer_project.textures_egui.get(index) {
                        let sized_texture =
                            egui::load::SizedTexture::new(texture_id, egui::vec2(100.0, 100.0));
                        ui.image(sized_texture);
                    } else {
                        ui.label("Not registered");
                    }
                });
            }

            ui.heading("Models");
            for model in renderer.renderer_project.models.iter() {
                ui.collapsing(RichText::from(&model.name).strong(), |ui| {
                    for (index, mesh) in model.meshes.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("Mesh {}", index));
                            ui.label(
                                RichText::from(format!("{} elements", mesh.num_elements)).weak(),
                            );
                        });
                    }
                });
            }

            ui.heading("Render Pipeline");
            ui.collapsing(RichText::from("Uniforms").strong(), |ui| {
                // Borrow only the storage mutably.
                let storage = &mut renderer
                    .renderer_project
                    .project_render_pipeline
                    .render_resource_storage;
                // Also extract an immutable reference to the queue.
                let queue = &renderer.queue;

                for render_bindings in &mut storage.render_bindings {
                    let index = render_bindings.set;
                    let type_name = get_render_binding_type(&render_bindings.provider_type);
                    let title = generate_two_part_title(
                        ui.style(),
                        &render_bindings.name,
                        format!("(index={index}, type={type_name})"),
                    );

                    ui.collapsing(title, |ui| match &mut render_bindings.provider_type {
                        BindingResourceType::Uniform(uniform_type) => match uniform_type {
                            UniformResourceType::Camera(uniform_buffer) => {
                                let mut data = *uniform_buffer.get();

                                let view_position_title = generate_two_part_title(
                                    ui.style(),
                                    "View Position",
                                    "(binding=0, type=vec4)",
                                );
                                ui.collapsing(view_position_title, |ui| {
                                    ui.horizontal(|ui| {
                                        for col in 0..4 {
                                            ui.add(egui::DragValue::new(
                                                &mut data.view_position[col],
                                            ))
                                            .changed();
                                        }
                                    });
                                });

                                let view_proj_title = generate_two_part_title(
                                    ui.style(),
                                    "View Projection",
                                    "(binding=1, type=mat4)",
                                );
                                ui.collapsing(view_proj_title, |ui| {
                                    for row in 0..4 {
                                        ui.horizontal(|ui| {
                                            for col in 0..4 {
                                                ui.add(egui::DragValue::new(
                                                    &mut data.view_proj[row][col],
                                                ))
                                                .changed();
                                            }
                                        });
                                    }
                                });
                            }
                            UniformResourceType::Time(uniform_buffer) => {
                                ui.horizontal(|ui| {
                                    let time_title = generate_two_part_title(
                                        ui.style(),
                                        "Time",
                                        "(binding=0, type=float)",
                                    );
                                    ui.collapsing(time_title, |ui| {
                                        let mut time = uniform_buffer.get().time;
                                        if ui
                                            .add(egui::DragValue::new(&mut time).speed(0.05))
                                            .changed()
                                        {
                                            uniform_buffer.write(queue, TimeUniformData::new(time));
                                        }
                                    });
                                });
                            }
                            UniformResourceType::Custom(custom_uniform) => match custom_uniform {
                                CustomUniform::Color(uniform_buffer) => {
                                    let title = generate_two_part_title(
                                        ui.style(),
                                        "Custom Color",
                                        "(binding=0, type=vec4)",
                                    );
                                    ui.collapsing(title, |ui| {
                                        ui_edit_uniform(ui, queue, uniform_buffer, edit_color);
                                    });
                                }
                                CustomUniform::Vec4(uniform_buffer) => {
                                    let title = generate_two_part_title(
                                        ui.style(),
                                        "Custom Vec4",
                                        "(binding=0, type=vec4)",
                                    );
                                    ui.collapsing(title, |ui| {
                                        ui_edit_uniform(ui, queue, uniform_buffer, edit_vec4);
                                    });
                                }
                                CustomUniform::Mat4(uniform_buffer) => {
                                    let title = generate_two_part_title(
                                        ui.style(),
                                        "Custom Mat4",
                                        "(binding=0, type=mat4)",
                                    );
                                    ui.collapsing(title, |ui| {
                                        ui_edit_uniform(ui, queue, uniform_buffer, edit_mat4);
                                    });
                                }
                            },
                        },
                        BindingResourceType::Texture(texture_index) => {
                            ui.label(format!("Texture {}", texture_index));
                        }
                    });
                }
            });
        });

    fn generate_two_part_title(
        style: &Style,
        strong: impl Into<String>,
        weak: impl Into<String>,
    ) -> LayoutJob {
        let mut title = LayoutJob::default();
        RichText::from(strong.into()).strong().append_to(
            &mut title,
            style,
            Default::default(),
            Default::default(),
        );
        title.append("", 5.0, Default::default());
        RichText::from(weak.into()).weak().append_to(
            &mut title,
            style,
            Default::default(),
            Default::default(),
        );
        title
    }

    fn get_render_binding_type(resource_type: &BindingResourceType) -> &'static str {
        match resource_type {
            BindingResourceType::Uniform(uniform) => match uniform {
                UniformResourceType::Camera(_) => "Camera",
                UniformResourceType::Time(_) => "Time",
                UniformResourceType::Custom(custom_uniform) => match custom_uniform {
                    CustomUniform::Color(_) => "Custom Color",
                    CustomUniform::Vec4(_) => "Custom Vec4",
                    CustomUniform::Mat4(_) => "Custom Mat4",
                },
            },
            BindingResourceType::Texture(_) => "Texture",
        }
    }
}

fn ui_edit_uniform<T: bytemuck::Pod + Copy>(
    ui: &mut egui::Ui,
    queue: &wgpu::Queue,
    uniform_buffer: &mut crate::renderer::uniform::UniformBuffer<T>,
    edit_fn: impl Fn(&mut egui::Ui, &mut T) -> bool,
) {
    let mut data = *uniform_buffer.get();
    if edit_fn(ui, &mut data) {
        uniform_buffer.write(queue, data);
    }
}

fn edit_color(ui: &mut egui::Ui, data: &mut [f32; 4]) -> bool {
    let mut rgba = egui::Rgba::from_rgba_premultiplied(data[0], data[1], data[2], data[3]);
    let response = egui::color_picker::color_edit_button_rgba(ui, &mut rgba, Alpha::OnlyBlend);
    if response.changed() {
        data[0] = rgba[0];
        data[1] = rgba[1];
        data[2] = rgba[2];
        data[3] = rgba[3];
        true
    } else {
        false
    }
}

fn edit_vec4(ui: &mut egui::Ui, data: &mut [f32; 4]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for v in data {
            if ui.add(egui::DragValue::new(v).speed(0.1)).changed() {
                changed = true;
            }
        }
    });
    changed
}

fn edit_mat4(ui: &mut egui::Ui, data: &mut [[f32; 4]; 4]) -> bool {
    let mut changed = false;
    for row in data {
        changed |= edit_vec4(ui, row);
    }
    changed
}
