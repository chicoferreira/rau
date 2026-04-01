use crate::{project::ModelId, ui::pane::StateSnapshot};
use egui_table::{AutoSizeMode, CellInfo, Column, HeaderCellInfo, HeaderRow, Table, TableDelegate};

impl StateSnapshot<'_> {
    pub fn model_inspector_ui(&mut self, ui: &mut egui::Ui, model_id: ModelId) {
        let Ok(model) = self.project.models.get(model_id) else {
            ui.label("Model couldn't be found.");
            return;
        };

        egui::CollapsingHeader::new(format!("Meshes ({})", model.meshes.len()))
            .default_open(true)
            .show(ui, |ui| {
                if model.meshes.is_empty() {
                    ui.weak("No meshes.");
                    return;
                }

                for (mesh_index, mesh) in model.meshes.iter().enumerate() {
                    let id = format!("model_mesh_{model_id:?}_{mesh_index}");
                    ui.push_id(id, |ui| {
                        ui.collapsing(format!("Mesh {mesh_index}"), |ui| {
                            let vertices = mesh.positions.len();
                            let normals = mesh.normals.len();
                            let uvs = mesh.texture_coords.len();
                            let tangents = mesh.tangents.len();
                            let bitangents = mesh.bitangents.len();
                            let indices = mesh.indices.len();
                            let triangles = indices / 3;

                            egui::Grid::new("mesh_grid")
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Vertices");
                                    ui.strong(vertices.to_string());
                                    ui.end_row();

                                    ui.label("Normals");
                                    ui.strong(normals.to_string());
                                    ui.end_row();

                                    ui.label("UVs");
                                    ui.strong(uvs.to_string());
                                    ui.end_row();

                                    ui.label("Tangents");
                                    ui.strong(tangents.to_string());
                                    ui.end_row();

                                    ui.label("Bitangents");
                                    ui.strong(bitangents.to_string());
                                    ui.end_row();

                                    ui.label("Indices");
                                    ui.strong(indices.to_string());
                                    ui.end_row();

                                    ui.label("Triangles");
                                    ui.strong(triangles.to_string());
                                    ui.end_row();

                                    ui.label("Material");
                                    ui.label(match mesh.material_id {
                                        None => "None".to_string(),
                                        Some(id) => model
                                            .materials
                                            .get(id)
                                            .map(|m| format!("{id}: {}", m.label))
                                            .unwrap_or_else(|| format!("{id}: <out of bounds>")),
                                    });
                                    ui.end_row();
                                });

                            ui.collapsing("Indices", |ui| {
                                if mesh.indices.len() < 3 {
                                    ui.weak("No indices.");
                                    return;
                                }

                                let row_count = mesh.indices.len();
                                let mut delegate = TriangleTableDelegate { mesh };

                                let columns = [
                                    Column::new(100.0).resizable(true),
                                    Column::new(250.0).resizable(true),
                                    Column::new(250.0).resizable(true),
                                    Column::new(250.0).resizable(true),
                                    Column::new(250.0).resizable(true),
                                    Column::new(250.0).resizable(true),
                                ];

                                ui.allocate_ui(egui::vec2(ui.available_width(), 320.0), |ui| {
                                    egui::Frame::new().inner_margin(6).show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        Table::new()
                                            .id_salt(("model_indices", model_id, mesh_index))
                                            .num_rows(row_count as u64)
                                            .headers([HeaderRow::new(18.0)])
                                            .columns(columns)
                                            .auto_size_mode(AutoSizeMode::Never)
                                            .show(ui, &mut delegate);
                                    });
                                });
                            });
                        });
                    });
                }
            });

        egui::CollapsingHeader::new(format!("Materials ({})", model.materials.len()))
            .default_open(true)
            .show(ui, |ui| {
                if model.materials.is_empty() {
                    ui.weak("No materials.");
                    return;
                }

                for (mat_index, mat) in model.materials.iter().enumerate() {
                    let id = format!("model_material_{model_id:?}_{mat_index}");
                    ui.push_id(id, |ui| {
                        ui.collapsing(format!("Material {mat_index}: {}", mat.label), |ui| {
                            if mat.texture_paths.is_empty() {
                                ui.weak("No textures referenced.");
                                return;
                            }

                            for (tex_index, tex) in mat.texture_paths.iter().enumerate() {
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

struct TriangleTableDelegate<'a> {
    mesh: &'a crate::project::model::Mesh,
}

impl TableDelegate for TriangleTableDelegate<'_> {
    fn header_cell_ui(&mut self, ui: &mut egui::Ui, cell: &HeaderCellInfo) {
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

    fn cell_ui(&mut self, ui: &mut egui::Ui, cell: &CellInfo) {
        let index = cell.row_nr as usize;
        let vi = self.mesh.indices.get(index).copied().map(|v| v as usize);

        ui.push_id(index, |ui| match cell.col_nr {
            0 => ui.weak(index.to_string()),
            1 => ui.label(
                vi.and_then(|vi| self.mesh.positions.get(vi))
                    .map(|&[x, y, z]| format!("{x:.3}, {y:.3}, {z:.3}"))
                    .unwrap_or("N/A".to_string()),
            ),
            2 => ui.label(
                vi.and_then(|vi| self.mesh.normals.get(vi))
                    .map(|&[x, y, z]| format!("{x:.3}, {y:.3}, {z:.3}"))
                    .unwrap_or("N/A".to_string()),
            ),
            3 => ui.label(
                vi.and_then(|vi| self.mesh.texture_coords.get(vi))
                    .map(|&[u, v]| format!("{u:.3}, {v:.3}"))
                    .unwrap_or("N/A".to_string()),
            ),
            4 => ui.label(
                vi.and_then(|vi| self.mesh.tangents.get(vi))
                    .map(|&[x, y, z]| format!("{x:.3}, {y:.3}, {z:.3}"))
                    .unwrap_or("N/A".to_string()),
            ),
            5 => ui.label(
                vi.and_then(|vi| self.mesh.bitangents.get(vi))
                    .map(|&[x, y, z]| format!("{x:.3}, {y:.3}, {z:.3}"))
                    .unwrap_or("N/A".to_string()),
            ),
            _ => unreachable!(),
        });
    }
}
