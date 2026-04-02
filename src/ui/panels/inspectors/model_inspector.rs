use egui::{Label, RichText, Sense};
use strum::IntoEnumIterator;

use crate::{
    project::{
        ModelId,
        model::{Model, vertex_buffer::VertexBufferField},
    },
    state::StateEvent,
    ui::{
        components::{hint::hint, selector::selectable_value},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn model_inspector_ui(&mut self, ui: &mut egui::Ui, model_id: ModelId) {
        let Ok(model) = self.project.models.get(model_id) else {
            ui.label("Model couldn't be found.");
            return;
        };

        model_vertex_buffer_spec_inspector_ui(ui, &mut self.pending_events, model_id, &model);

        egui::CollapsingHeader::new(format!("Meshes ({})", model.meshes().len()))
            .default_open(true)
            .show(ui, |ui| {
                if model.meshes().is_empty() {
                    ui.weak("No meshes.");
                    return;
                }

                for (mesh_index, mesh) in model.meshes().iter().enumerate() {
                    let id = format!("model_mesh_{model_id:?}_{mesh_index}");
                    ui.push_id(id, |ui| {
                        ui.collapsing(format!("Mesh {mesh_index}"), |ui| {
                            egui::Grid::new("mesh_grid")
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Vertices");
                                    ui.strong(mesh.positions().len().to_string());
                                    ui.end_row();

                                    ui.label("Normals");
                                    ui.strong(mesh.normals().len().to_string());
                                    ui.end_row();

                                    ui.label("UVs");
                                    ui.strong(mesh.texture_coords().len().to_string());
                                    ui.end_row();

                                    ui.label("Tangents");
                                    ui.strong(mesh.tangents().len().to_string());
                                    ui.end_row();

                                    ui.label("Bitangents");
                                    ui.strong(mesh.bitangents().len().to_string());
                                    ui.end_row();

                                    ui.label("Indices");
                                    ui.strong(mesh.indices().len().to_string());
                                    ui.end_row();

                                    ui.label("Triangles");
                                    ui.strong((mesh.indices().len() / 3).to_string());
                                    ui.end_row();

                                    ui.label("Material");
                                    ui.label(match mesh.material_index() {
                                        None => "None".to_string(),
                                        Some(id) => model
                                            .materials()
                                            .get(id)
                                            .map(|m| format!("{id}: {}", m.label()))
                                            .unwrap_or_else(|| format!("{id}: <out of bounds>")),
                                    });
                                    ui.end_row();
                                });

                            ui.collapsing("Indices", |ui| {
                                let row_count = mesh.indices().len();
                                if row_count < 3 {
                                    ui.weak("No indices.");
                                    return;
                                }

                                let mut delegate = TriangleTableDelegate { mesh };

                                let columns = [
                                    egui_table::Column::new(100.0).resizable(true),
                                    egui_table::Column::new(300.0).resizable(true),
                                    egui_table::Column::new(300.0).resizable(true),
                                    egui_table::Column::new(300.0).resizable(true),
                                    egui_table::Column::new(300.0).resizable(true),
                                    egui_table::Column::new(300.0).resizable(true),
                                ];

                                ui.allocate_ui(egui::vec2(ui.available_width(), 320.0), |ui| {
                                    egui::Frame::new().inner_margin(6).show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        egui_table::Table::new()
                                            .id_salt(("model_indices", model_id, mesh_index))
                                            .num_rows(row_count as u64)
                                            .headers([egui_table::HeaderRow::new(18.0)])
                                            .columns(columns)
                                            .auto_size_mode(egui_table::AutoSizeMode::Never)
                                            .show(ui, &mut delegate);
                                    });
                                });
                            });
                        });
                    });
                }
            });

        egui::CollapsingHeader::new(format!("Materials ({})", model.materials().len()))
            .default_open(true)
            .show(ui, |ui| {
                if model.materials().is_empty() {
                    ui.weak("No materials.");
                    return;
                }

                for (mat_index, mat) in model.materials().iter().enumerate() {
                    let id = format!("model_material_{model_id:?}_{mat_index}");
                    ui.push_id(id, |ui| {
                        ui.collapsing(format!("Material {mat_index}: {}", mat.label()), |ui| {
                            if mat.texture_paths().is_empty() {
                                ui.weak("No textures referenced.");
                                return;
                            }

                            for (tex_index, tex) in mat.texture_paths().iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.weak(format!("{tex_index}"));
                                    ui.label(tex);
                                });
                            }
                        });
                    });
                }
            });
    }
}
fn model_vertex_buffer_spec_inspector_ui(
    ui: &mut egui::Ui,
    pending_events: &mut Vec<StateEvent>,
    model_id: ModelId,
    model: &Model,
) {
    let fields: Vec<VertexBufferField> = model.vertex_buffer_spec().fields.clone();

    egui::CollapsingHeader::new("Vertex Buffer Layout")
        .default_open(false)
        .show(ui, |ui| {
            let response = egui_dnd::dnd(ui, ("model_vertex_buffer_spec", model_id)).show_custom(
                |ui, iter| {
                    for (index, field) in fields.iter().enumerate() {
                        if index != 0 {
                            ui.add_space(5.0);
                        }
                        let item_id = egui::Id::new((model_id, "vertex_buffer_field", index));
                        ui.push_id(index, |ui| {
                            iter.next(ui, item_id, index, true, |ui, item_handle| {
                                item_handle.ui(ui, |ui, handle, _state| {
                                    ui.horizontal(|ui| {
                                        handle.ui(ui, |ui| {
                                            ui.add(
                                                Label::new(format!("Location {index}"))
                                                    .selectable(false)
                                                    .sense(Sense::click()),
                                            )
                                            .context_menu(|ui| {
                                                if ui.button("Delete attribute").clicked() {
                                                    pending_events.push(
                                                        StateEvent::DeleteModelVertexBufferField(
                                                            model_id, index,
                                                        ),
                                                    );
                                                    ui.close();
                                                }
                                            });
                                        });

                                        let mut current = *field;
                                        selectable_value(
                                            ui,
                                            ("vertex_buffer_field_kind", model_id, index),
                                            &mut current,
                                            |f| f.to_string(),
                                            VertexBufferField::iter(),
                                        );
                                        if current != *field {
                                            pending_events.push(
                                                StateEvent::UpdateModelVertexBufferField(
                                                    model_id, index, current,
                                                ),
                                            );
                                        }
                                    });
                                })
                            });
                        });
                    }
                },
            );

            if let Some(update) = response.final_update() {
                pending_events.push(StateEvent::ReorderModelVertexBufferField(model_id, update));
            }

            ui.add_space(6.0);

            ui.menu_button("Add attribute", |ui| {
                for kind in VertexBufferField::iter() {
                    if ui.button(kind.to_string()).clicked() {
                        ui.close();
                        pending_events.push(StateEvent::AddModelVertexBufferField(model_id, kind));
                    }
                }
            });

            if !fields.is_empty() {
                ui.add_space(6.0);
                ui.add(hint(|ui| {
                    ui.label("Right-click a");
                    ui.label(RichText::new("Location").strong());
                    ui.label("label to remove an attribute, or drag it to reorder.");
                }));
            }
        });
}

struct TriangleTableDelegate<'a> {
    mesh: &'a crate::project::model::Mesh,
}

impl egui_table::TableDelegate for TriangleTableDelegate<'_> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::HeaderCellInfo) {
        let title = match cell.col_range.start {
            0 => "Index",
            1 => "Position",
            2 => "Normal",
            3 => "UV",
            4 => "Tangent",
            5 => "Bitangent",
            _ => "",
        };
        ui.strong(title);
    }

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell: &egui_table::CellInfo) {
        let index = cell.row_nr as usize;
        let vi = self.mesh.indices().get(index).copied().map(|v| v as usize);

        fn format_vec<const N: usize>(v: &[f32; N]) -> String {
            // TODO: move to std when https://github.com/rust-lang/rust/issues/48919 gets merged
            itertools::Itertools::intersperse(
                v.iter().copied().map(|f| format!("{:.3}", f)),
                ", ".to_string(),
            )
            .collect::<String>()
        }

        fn format_cell<const N: usize>(content: &[[f32; N]], index: Option<usize>) -> String {
            index
                .and_then(|i| content.get(i).map(|v| format_vec(v)))
                .unwrap_or("N/A".to_string())
        }

        ui.push_id(index, |ui| match cell.col_nr {
            0 => ui.weak(index.to_string()),
            1 => ui.label(format_cell(self.mesh.positions(), vi)),
            2 => ui.label(format_cell(self.mesh.normals(), vi)),
            3 => ui.label(format_cell(self.mesh.texture_coords(), vi)),
            4 => ui.label(format_cell(self.mesh.tangents(), vi)),
            5 => ui.label(format_cell(self.mesh.bitangents(), vi)),
            _ => unreachable!(),
        });
    }
}
