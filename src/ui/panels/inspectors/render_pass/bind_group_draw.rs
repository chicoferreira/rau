use std::ops::Range;

use egui::{CollapsingHeader, Grid, Label, RichText, Sense};
use strum::IntoEnumIterator;

use crate::{
    project::{
        BindGroupId, RenderPassId,
        bindgroup::BindGroup,
        model::Model,
        render_pass::{RenderDraw, RenderPipeline},
        storage::Storage,
    },
    ui::components::{
        hint::hint,
        selector::{AsWidgetText, ComboBoxExt},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter)]
enum DrawKind {
    Model,
    Direct,
}

impl AsWidgetText for DrawKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            DrawKind::Model => "Model",
            DrawKind::Direct => "Direct",
        }
        .into()
    }
}

fn render_draw_kind(draw: &RenderDraw) -> DrawKind {
    match draw {
        RenderDraw::Model { .. } => DrawKind::Model,
        RenderDraw::Direct { .. } => DrawKind::Direct,
    }
}

fn draw_default(kind: DrawKind) -> RenderDraw {
    match kind {
        DrawKind::Model => RenderDraw::Model {
            model_id: None,
            instances: 0..1,
            mesh_vertex_slot: 0,
            material_bind_group_slot: None,
        },
        DrawKind::Direct => RenderDraw::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
    }
}

fn range_u32_edit(ui: &mut egui::Ui, range: &mut Range<u32>) {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut range.start).range(0..=range.end));
        ui.label("..");
        ui.add(egui::DragValue::new(&mut range.end).range(range.start..=u32::MAX));
    });
}

fn combo_list<T: Copy + PartialEq + AsWidgetText, I: IntoIterator<Item = T>>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    options: I,
    value: &mut T,
) -> bool {
    let before = *value;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(value.as_widget_text())
        .show_ui_list(ui, options, value);
    *value != before
}

#[derive(Clone, Copy)]
enum UnifiedEntry {
    Static { slot: u32, bg_id: BindGroupId },
    Material { slot: u32 },
}

fn first_available_slot(used: impl IntoIterator<Item = u32>) -> u32 {
    let mut sorted: Vec<u32> = used.into_iter().collect();
    sorted.sort_unstable();
    let mut candidate = 0u32;
    for s in sorted {
        if s == candidate {
            candidate = candidate.saturating_add(1);
        } else if s > candidate {
            break;
        }
    }
    candidate
}

pub fn bind_groups_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    pipeline: &mut RenderPipeline,
    bind_groups: &Storage<BindGroup>,
) {
    let static_before = pipeline.static_bind_groups.clone();
    let draw_before = pipeline.draw.clone();

    let mut entries: Vec<UnifiedEntry> = pipeline
        .static_bind_groups
        .iter()
        .cloned()
        .map(|(slot, bg_id)| UnifiedEntry::Static { slot, bg_id })
        .collect();

    if let RenderDraw::Model {
        material_bind_group_slot: Some(slot),
        ..
    } = pipeline.draw
    {
        entries.push(UnifiedEntry::Material { slot });
    }

    entries.sort_by_key(|entry| match entry {
        UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => *slot,
    });

    let mut delete_index: Option<usize> = None;
    let mut remove_material = false;

    CollapsingHeader::new(format!("Bind Groups ({})", entries.len()))
        .default_open(true)
        .show(ui, |ui| {
            let response =
                egui_dnd::dnd(ui, (render_pass_id, pipeline.id, "static_bind_groups")).show_custom(
                    |ui, iter| {
                        for (index, entry) in entries.iter_mut().enumerate() {
                            if index != 0 {
                                ui.add_space(5.0);
                            }
                            let item_id =
                                egui::Id::new((render_pass_id, pipeline.id, "sbg", index));
                            ui.push_id(index, |ui| {
                                iter.next(ui, item_id, index, true, |ui, item_handle| {
                                    item_handle.ui(ui, |ui, handle, _state| {
                                        handle.ui(ui, |ui| {
                                            ui.add(
                                                Label::new(match entry {
                                                    UnifiedEntry::Static { slot, .. } => {
                                                        format!("Slot {}", slot)
                                                    }
                                                    UnifiedEntry::Material { slot } => {
                                                        format!("Slot {} (Material)", slot)
                                                    }
                                                })
                                                .selectable(false)
                                                .sense(Sense::click()),
                                            )
                                            .context_menu(|ui| match entry {
                                                UnifiedEntry::Static { .. } => {
                                                    if ui.button("Delete Bind Group").clicked() {
                                                        delete_index = Some(index);
                                                        ui.close();
                                                    }
                                                }
                                                UnifiedEntry::Material { .. } => {
                                                    if ui
                                                        .button("Remove Material Bind Group")
                                                        .clicked()
                                                    {
                                                        remove_material = true;
                                                        ui.close();
                                                    }
                                                }
                                            });
                                        });

                                        ui.indent(
                                            ("static_bind_group_fields", pipeline.id, index),
                                            |ui| {
                                                Grid::new((
                                                    "pipeline_bind_groups_grid",
                                                    render_pass_id,
                                                    pipeline.id,
                                                    index,
                                                ))
                                                .num_columns(2)
                                                .show(ui, |ui| match entry {
                                                    UnifiedEntry::Static { slot, bg_id } => {
                                                        ui.label("Slot");
                                                        ui.add(
                                                            egui::DragValue::new(slot).speed(0.1),
                                                        );
                                                        ui.end_row();

                                                        let bg_before = *bg_id;
                                                        let mut selected = Some(*bg_id);
                                                        ui.label("Bind Group");
                                                        egui::ComboBox::from_id_salt((
                                                            "static_bg",
                                                            pipeline.id,
                                                            index,
                                                        ))
                                                        .selected_text_storage_opt(
                                                            bind_groups,
                                                            selected,
                                                        )
                                                        .show_ui_storage_opt(
                                                            ui,
                                                            bind_groups,
                                                            &mut selected,
                                                        );
                                                        ui.end_row();

                                                        if let Some(new_id) = selected {
                                                            *bg_id = new_id;
                                                        } else {
                                                            *bg_id = bg_before;
                                                        }
                                                    }
                                                    UnifiedEntry::Material { slot } => {
                                                        ui.label("Slot");
                                                        ui.add(
                                                            egui::DragValue::new(slot).speed(0.1),
                                                        );
                                                        ui.end_row();

                                                        ui.label("");
                                                        ui.label(
                                                            RichText::new("From RenderDraw::Model")
                                                                .italics(),
                                                        );
                                                        ui.end_row();
                                                    }
                                                });
                                            },
                                        );
                                    })
                                });
                            });
                        }
                    },
                );

            if let Some(update) = response.final_update() {
                egui_dnd::utils::shift_vec(update.from, update.to, &mut entries);
                for (i, entry) in entries.iter_mut().enumerate() {
                    match entry {
                        UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => {
                            *slot = i as u32;
                        }
                    }
                }
            }

            ui.add_space(6.0);

            let can_add = bind_groups.list().next().is_some();
            if ui
                .add_enabled(can_add, egui::Button::new("Add Bind Group"))
                .clicked()
            {
                let next_slot = first_available_slot(entries.iter().map(|entry| match entry {
                    UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => *slot,
                }));

                let first_bg = bind_groups
                    .list()
                    .next()
                    .map(|(id, _)| id)
                    .expect("can_add implies at least one bind group");

                entries.push(UnifiedEntry::Static {
                    slot: next_slot,
                    bg_id: first_bg,
                });
            }

            if let RenderDraw::Model {
                material_bind_group_slot: None,
                ..
            } = pipeline.draw
            {
                ui.add_space(4.0);
                if ui
                    .button("Add Material Bind Group")
                    .on_hover_text("Create a material bind group entry tied to the model draw")
                    .clicked()
                {
                    let next_slot = first_available_slot(entries.iter().map(|entry| match entry {
                        UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => {
                            *slot
                        }
                    }));

                    entries.push(UnifiedEntry::Material { slot: next_slot });
                }
            }

            if let Some(index) = delete_index
                && index < entries.len()
            {
                entries.remove(index);
            }

            if remove_material {
                entries.retain(|entry| !matches!(entry, UnifiedEntry::Material { .. }));
            }

            if !entries.is_empty() {
                ui.add_space(6.0);
                ui.add(hint(|ui| {
                    ui.label("Right-click a");
                    ui.label(RichText::new("Slot").strong());
                    ui.label("to delete it, or drag to reorder.");
                }));
            }
        });

    let new_static: Vec<(u32, BindGroupId)> = entries
        .iter()
        .filter_map(|entry| match entry {
            UnifiedEntry::Static { slot, bg_id } => Some((*slot, *bg_id)),
            UnifiedEntry::Material { .. } => None,
        })
        .collect();

    let new_material_slot: Option<u32> = entries.iter().find_map(|entry| match entry {
        UnifiedEntry::Material { slot } => Some(*slot),
        _ => None,
    });

    if new_static != static_before {
        pipeline.set_static_bind_groups(new_static);
    }

    if let RenderDraw::Model {
        material_bind_group_slot,
        ..
    } = &pipeline.draw
    {
        if *material_bind_group_slot != new_material_slot {
            let mut edited = draw_before;
            if let RenderDraw::Model {
                material_bind_group_slot,
                ..
            } = &mut edited
            {
                *material_bind_group_slot = new_material_slot;
            }
            pipeline.set_draw(edited);
        }
    }
}

pub fn draw_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    pipeline: &mut RenderPipeline,
    models: &Storage<Model>,
) {
    let before = pipeline.draw.clone();

    let mut kind = render_draw_kind(&pipeline.draw);
    let kind_before = kind;

    let mut edited = before.clone();

    CollapsingHeader::new("Draw")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(("pipeline_draw_model_grid", render_pass_id, pipeline.id))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Draw Kind");
                    combo_list(ui, ("draw_kind", pipeline.id), DrawKind::iter(), &mut kind);
                    ui.end_row();

                    if kind != kind_before {
                        edited = draw_default(kind);
                        if let RenderDraw::Model {
                            material_bind_group_slot,
                            ..
                        } = &mut edited
                        {
                            *material_bind_group_slot = Some(first_available_slot(
                                pipeline.static_bind_groups.iter().map(|(slot, _)| *slot),
                            ));
                        }
                    }

                    match &mut edited {
                        RenderDraw::Model {
                            model_id,
                            instances,
                            mesh_vertex_slot,
                            material_bind_group_slot: _,
                        } => {
                            ui.label("Model");
                            egui::ComboBox::from_id_salt(("draw_model_id", pipeline.id))
                                .selected_text_storage_opt(models, *model_id)
                                .show_ui_storage_opt_with_none(ui, models, model_id);
                            ui.end_row();

                            ui.label("Instances");
                            range_u32_edit(ui, instances);
                            ui.end_row();

                            ui.label("Mesh Vertex Slot");
                            ui.add(egui::DragValue::new(mesh_vertex_slot).speed(0.1));
                            ui.end_row();
                        }
                        RenderDraw::Direct {
                            vertices,
                            instances,
                        } => {
                            ui.label("Vertices");
                            range_u32_edit(ui, vertices);
                            ui.end_row();

                            ui.label("Instances");
                            range_u32_edit(ui, instances);
                            ui.end_row();
                        }
                    }
                });
        });

    if edited != before {
        pipeline.set_draw(edited);
    }
}
