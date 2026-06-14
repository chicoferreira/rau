use egui::RichText;
use strum::IntoEnumIterator;

use crate::{
    project::{
        BindGroupId, ModelId,
        paths::FilePath,
        resource::{
            bindgroup::BindGroup,
            model::{
                Material, Mesh, MeshMaterialSelection, Model, ModelRuntime, TextureType,
                vertex_buffer::{VertexBufferField, VertexBufferSpec},
            },
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector::{self, AsWidgetText},
        },
        pane::StateSnapshot,
    },
    utils::{event_queue::EventQueue, shader_preview::ShaderGenCtx},
    workspace::StateEvent,
};

impl StateSnapshot<'_> {
    pub fn model_inspector_ui(&mut self, ui: &mut egui::Ui, model_id: ModelId) {
        let Ok(model) = self.project.models.get_mut(model_id) else {
            ui.label("Model couldn't be found.");
            return;
        };

        model_source_ui(ui, model, self.file_storage.files());

        let mut material_bind_group_ids = model.material_bind_group_ids().to_vec();
        let mut mesh_material_selections = model.mesh_material_selections().to_vec();
        let mut vertex_buffer_spec = model.vertex_buffer_spec().clone();

        model_vertex_buffer_spec_inspector_ui(ui, model_id, &mut vertex_buffer_spec);

        match self.runtime_project.models.get_init(model_id) {
            Ok(Some(model_runtime)) => {
                meshes_ui(ui, model_id, model_runtime, &mut mesh_material_selections);
                materials_ui(
                    ui,
                    model_id,
                    model_runtime,
                    &self.project.bind_groups,
                    &mut material_bind_group_ids,
                    self.event_queue,
                );
            }
            Ok(None) => {
                ui.spinner();
            }
            Err(_) => {}
        }

        model.set_material_bind_group_ids(material_bind_group_ids);
        model.set_mesh_material_selections(mesh_material_selections);
        model.set_vertex_buffer_spec(vertex_buffer_spec);

        if let Ok(model) = self.project.models.get(model_id) {
            let ctx = ShaderGenCtx::from_project(self.project);
            shader_code_section(ui, model, &ctx);
        }
    }
}

fn model_source_ui(ui: &mut egui::Ui, model: &mut Model, files: Option<&[FilePath]>) {
    inspector::section(ui, "Source", |ui| {
        inspector::field_grid(ui, "model_inspector_grid", |ui| {
            let mut source = model.source().cloned();

            let Some(files) = files else {
                ui.spinner();
                return;
            };

            if inspector::file_combo_row(ui, "Source", "model_source", files, &mut source, |path| {
                path.extension() == Some("obj")
            }) {
                model.set_source(source);
            }
        });
    });
}

fn model_vertex_buffer_spec_inspector_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    vertex_buffer_spec: &mut VertexBufferSpec,
) {
    let before = vertex_buffer_spec.fields.clone();
    let mut entries = before
        .iter()
        .copied()
        .enumerate()
        .collect::<Vec<(usize, VertexBufferField)>>();

    inspector::section(ui, "Vertex Buffer Layout", |ui| {
        let mut list_edits = draggable_list(
            ui,
            ("model_vertex_buffer_spec", model_id),
            &entries,
            |ui, (entry_id, field), index, handle, list_edits| {
                model_vertex_buffer_field_ui(ui, handle, index, *entry_id, *field, list_edits);
            },
        );

        ui.add_space(6.0);

        ui.menu_button("Add attribute", |ui| {
            for kind in VertexBufferField::iter() {
                if ui.button(kind.to_string()).clicked() {
                    ui.close();
                    let next_entry_id = entries
                        .iter()
                        .map(|(entry_id, _)| *entry_id)
                        .max()
                        .map(|entry_id| entry_id + 1)
                        .unwrap_or_default();
                    list_edits.push_add_edit((next_entry_id, kind));
                }
            }
        });

        if !entries.is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click a");
                ui.label(RichText::new("Location").strong());
                ui.label("label to remove an attribute, or drag it to reorder.");
            }));
        }

        list_edits.apply(&mut entries);
        let fields = entries.iter().map(|(_, field)| *field).collect::<Vec<_>>();

        if fields != before {
            vertex_buffer_spec.fields = fields;
        }
    });
}

fn model_vertex_buffer_field_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    entry_id: usize,
    field: VertexBufferField,
    list_edits: &mut ListEdits<(usize, VertexBufferField)>,
) {
    handle.ui(ui, |ui| {
        ui.add(egui::Label::new(format!("Location {index}")).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Delete attribute").clicked() {
                    list_edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    ui.indent(("model_vertex_buffer_field", index), |ui| {
        let mut current = field;
        inspector::field_grid(ui, ("model_vertex_buffer_field_grid", index), |ui| {
            inspector::combo_row(
                ui,
                "Attribute",
                "vertex_buffer_field_kind",
                VertexBufferField::iter(),
                &mut current,
            );
        });

        if current != field {
            list_edits.push_set_edit(index, (entry_id, current));
        }
    });
}

fn meshes_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    model_runtime: &ModelRuntime,
    mesh_material_selections: &mut Vec<MeshMaterialSelection>,
) {
    inspector::section(
        ui,
        &format!("Meshes ({})", model_runtime.meshes().len()),
        |ui| {
            if model_runtime.meshes().is_empty() {
                ui.weak("No meshes.");
                return;
            }

            for (mesh_index, mesh) in model_runtime.meshes().iter().enumerate() {
                let id = format!("model_mesh_{model_id:?}_{mesh_index}");
                ui.push_id(id, |ui| {
                    mesh_ui(
                        ui,
                        model_id,
                        mesh_index,
                        mesh,
                        model_runtime,
                        mesh_material_selections,
                    );
                });
            }
        },
    );
}

fn mesh_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    mesh_index: usize,
    mesh: &Mesh,
    model_runtime: &ModelRuntime,
    mesh_material_selections: &mut Vec<MeshMaterialSelection>,
) {
    ui.collapsing(format!("Mesh {mesh_index}"), |ui| {
        mesh_info_ui(
            ui,
            model_id,
            mesh_index,
            mesh,
            model_runtime,
            mesh_material_selections,
        );
        mesh_indices_ui(ui, model_id, mesh_index, mesh);
    });
}

fn mesh_info_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    mesh_index: usize,
    mesh: &Mesh,
    model_runtime: &ModelRuntime,
    mesh_material_selections: &mut Vec<MeshMaterialSelection>,
) {
    inspector::field_grid(ui, "mesh_grid", |ui| {
        inspector::row(ui, "Vertices", |ui| {
            ui.strong(mesh.positions().len().to_string());
        });
        inspector::row(ui, "Normals", |ui| {
            ui.strong(mesh.normals().len().to_string());
        });
        inspector::row(ui, "UVs", |ui| {
            ui.strong(mesh.texture_coords().len().to_string());
        });
        inspector::row(ui, "Tangents", |ui| {
            ui.strong(mesh.tangents().len().to_string());
        });
        inspector::row(ui, "Bitangents", |ui| {
            ui.strong(mesh.bitangents().len().to_string());
        });
        inspector::row(ui, "Indices", |ui| {
            ui.strong(mesh.indices().len().to_string());
        });
        inspector::row(ui, "Triangles", |ui| {
            ui.strong((mesh.indices().len() / 3).to_string());
        });
        inspector::row(ui, "Material", |ui| {
            mesh_material_selection_ui(
                ui,
                model_id,
                mesh_index,
                mesh,
                model_runtime,
                mesh_material_selections,
            );
        });
    });
}

fn mesh_material_selection_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    mesh_index: usize,
    mesh: &Mesh,
    model_runtime: &ModelRuntime,
    mesh_material_selections: &mut Vec<MeshMaterialSelection>,
) {
    let current_selection = mesh_material_selection(mesh_material_selections, mesh_index);
    let mut selection = current_selection.clone();
    let materials = model_runtime.materials();
    let source_index = mesh.material_index();

    let options = [
        MeshMaterialSelection::FromSource,
        MeshMaterialSelection::Material(None),
    ]
    .into_iter()
    .chain((0..materials.len()).map(|i| MeshMaterialSelection::Material(Some(i))));

    inspector::value_combo_with(
        ui,
        ("mesh_material_selection", model_id, mesh_index),
        options,
        |sel| material_selection_label(sel, source_index, materials).into(),
        &mut selection,
    );

    if selection != current_selection {
        set_mesh_material_selection(mesh_material_selections, mesh_index, selection);
    }
}

fn mesh_indices_ui(ui: &mut egui::Ui, model_id: ModelId, mesh_index: usize, mesh: &Mesh) {
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
}

fn materials_ui(
    ui: &mut egui::Ui,
    model_id: ModelId,
    model_runtime: &ModelRuntime,
    bind_groups: &Storage<BindGroup>,
    material_bind_group_ids: &mut Vec<Option<BindGroupId>>,
    event_queue: &mut EventQueue<StateEvent>,
) {
    inspector::section(
        ui,
        &format!("Materials ({})", model_runtime.materials().len()),
        |ui| {
            if model_runtime.materials().is_empty() {
                ui.weak("No materials.");
                return;
            }

            if ui.button("Derive Bind Groups from Materials…").clicked() {
                event_queue.add(StateEvent::OpenMaterialBindGroupsModal(model_id));
            }

            ui.add_space(3.0);

            for (mat_index, mat) in model_runtime.materials().iter().enumerate() {
                let id = format!("model_material_{model_id:?}_{mat_index}");
                ui.push_id(id, |ui| {
                    material_ui(
                        ui,
                        mat_index,
                        mat,
                        bind_groups,
                        material_bind_group_ids,
                        event_queue,
                    )
                });
            }
        },
    );
}

fn material_ui(
    ui: &mut egui::Ui,
    mat_index: usize,
    mat: &Material,
    bind_groups: &Storage<BindGroup>,
    material_bind_group_ids: &mut Vec<Option<BindGroupId>>,
    event_queue: &mut EventQueue<StateEvent>,
) {
    ui.collapsing(format!("Material {mat_index}: {}", mat.label()), |ui| {
        material_bind_group_ui(ui, mat_index, bind_groups, material_bind_group_ids);
        material_textures_ui(ui, mat, event_queue);
    });
}

fn material_bind_group_ui(
    ui: &mut egui::Ui,
    mat_index: usize,
    bind_groups: &Storage<BindGroup>,
    material_bind_group_ids: &mut Vec<Option<BindGroupId>>,
) {
    inspector::field_grid(ui, "model_material_grid", |ui| {
        let current_bind_group_id = material_bind_group_id(material_bind_group_ids, mat_index);
        let mut bind_group_id = current_bind_group_id;

        if inspector::storage_opt_combo_row(
            ui,
            "Bind Group",
            "model_material_bind_group",
            bind_groups,
            &mut bind_group_id,
        ) {
            set_material_bind_group_id(material_bind_group_ids, mat_index, bind_group_id);
        }
    });
}

fn material_textures_ui(
    ui: &mut egui::Ui,
    mat: &Material,
    event_queue: &mut EventQueue<StateEvent>,
) {
    egui::CollapsingHeader::new("Textures")
        .default_open(true)
        .show(ui, |ui| {
            inspector::field_grid(ui, "model_material_textures_grid", |ui| {
                for texture_type in TextureType::iter() {
                    inspector::row(ui, texture_type.to_string(), |ui| {
                        match mat.get_texture_path(texture_type) {
                            Some(path) => {
                                let label =
                                    egui::Label::new(path.to_string()).sense(egui::Sense::click());
                                ui.add(label).context_menu(|ui| {
                                    if ui.button("Derive Texture").clicked() {
                                        let event = StateEvent::DeriveTextureFromPath {
                                            path: path.clone(),
                                            texture_type,
                                        };
                                        event_queue.add(event);
                                        ui.close();
                                    }
                                });
                            }
                            None => {
                                ui.weak("—");
                            }
                        }
                    });
                }
            });
        });
}

fn material_bind_group_id(
    material_bind_group_ids: &[Option<BindGroupId>],
    material_index: usize,
) -> Option<BindGroupId> {
    material_bind_group_ids
        .get(material_index)
        .copied()
        .flatten()
}

fn set_material_bind_group_id(
    material_bind_group_ids: &mut Vec<Option<BindGroupId>>,
    material_index: usize,
    bind_group_id: Option<BindGroupId>,
) {
    if material_bind_group_ids.len() <= material_index {
        material_bind_group_ids.resize(material_index + 1, None);
    }

    material_bind_group_ids[material_index] = bind_group_id;
}

fn mesh_material_selection(
    mesh_material_selections: &[MeshMaterialSelection],
    mesh_index: usize,
) -> MeshMaterialSelection {
    mesh_material_selections
        .get(mesh_index)
        .cloned()
        .unwrap_or_default()
}

fn set_mesh_material_selection(
    mesh_material_selections: &mut Vec<MeshMaterialSelection>,
    mesh_index: usize,
    selection: MeshMaterialSelection,
) {
    if mesh_material_selections.len() <= mesh_index {
        mesh_material_selections.resize(mesh_index + 1, MeshMaterialSelection::default());
    }

    mesh_material_selections[mesh_index] = selection;
}

struct TriangleTableDelegate<'a> {
    mesh: &'a Mesh,
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

impl AsWidgetText for VertexBufferField {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.to_string().into()
    }
}

fn material_selection_label(
    selection: &MeshMaterialSelection,
    source_index: Option<usize>,
    materials: &[Material],
) -> String {
    match selection {
        MeshMaterialSelection::FromSource => match source_index {
            None => "From Source (none)".to_string(),
            Some(id) => materials
                .get(id)
                .map(|m| format!("From Source ({id}: {})", m.label()))
                .unwrap_or_else(|| format!("From Source ({id}: <out of bounds>)")),
        },
        MeshMaterialSelection::Material(None) => "None".to_string(),
        MeshMaterialSelection::Material(Some(i)) => materials
            .get(*i)
            .map(|m| format!("{i}: {}", m.label()))
            .unwrap_or_else(|| format!("{i}: <out of bounds>")),
    }
}
