use std::ops::Range;

use egui::{CollapsingHeader, Grid, Label, RichText, Sense};
use strum::IntoEnumIterator;

use crate::{
    project::{
        BindGroupId, RenderPassId,
        bindgroup::BindGroup,
        model::Model,
        render_pass::{MAX_RENDER_PASS_BIND_GROUPS, RenderDraw, RenderPipeline},
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

#[derive(Clone, Copy)]
enum UnifiedEntry {
    Static {
        slot: u32,
        bg_id: Option<BindGroupId>,
    },
    Material {
        slot: u32,
    },
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
    let mut entries = collect_bind_group_entries(pipeline);
    let mut actions = BindGroupEntryActions::default();

    CollapsingHeader::new(format!("Bind Groups ({})", entries.len()))
        .default_open(true)
        .show(ui, |ui| {
            bind_group_entries_list_ui(
                ui,
                render_pass_id,
                pipeline.id,
                bind_groups,
                &mut entries,
                &mut actions,
            );
            bind_group_entries_footer_ui(ui, pipeline, &mut entries);

            apply_bind_group_entry_actions(&mut entries, actions);

            if !entries.is_empty() {
                bind_group_entries_hint_ui(ui);
            }
        });

    let (new_static, new_material_slot) = split_bind_group_entries(&entries);

    if new_static != static_before {
        pipeline.set_static_bind_groups(new_static);
    }

    apply_material_bind_group_slot(pipeline, draw_before, new_material_slot);
}

fn collect_bind_group_entries(pipeline: &RenderPipeline) -> Vec<UnifiedEntry> {
    let mut entries: Vec<_> = pipeline
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
    entries
}

#[derive(Default)]
struct BindGroupEntryActions {
    delete_index: Option<usize>,
    remove_material: bool,
}

fn bind_group_entries_list_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    pipeline_id: usize,
    bind_groups: &Storage<BindGroup>,
    entries: &mut Vec<UnifiedEntry>,
    actions: &mut BindGroupEntryActions,
) {
    let response = egui_dnd::dnd(ui, (render_pass_id, pipeline_id, "static_bind_groups"))
        .show_custom(|ui, iter| {
            for (index, entry) in entries.iter_mut().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                let item_id = egui::Id::new((render_pass_id, pipeline_id, "sbg", index));
                ui.push_id(index, |ui| {
                    iter.next(ui, item_id, index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            bind_group_entry_row_ui(
                                ui,
                                handle,
                                render_pass_id,
                                pipeline_id,
                                bind_groups,
                                index,
                                entry,
                                actions,
                            );
                        })
                    });
                });
            }
        });

    if let Some(update) = response.final_update() {
        egui_dnd::utils::shift_vec(update.from, update.to, entries);
        normalize_entry_slots(entries);
    }
}

fn bind_group_entry_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    render_pass_id: RenderPassId,
    pipeline_id: usize,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    entry: &mut UnifiedEntry,
    actions: &mut BindGroupEntryActions,
) {
    handle.ui(ui, |ui| {
        bind_group_entry_title_ui(ui, entry, index, actions);

        ui.indent(("static_bind_group_fields", pipeline_id, index), |ui| {
            bind_group_entry_fields_ui(ui, render_pass_id, pipeline_id, bind_groups, index, entry);
        });
    });
}

fn bind_group_entry_title_ui(
    ui: &mut egui::Ui,
    entry: &UnifiedEntry,
    index: usize,
    actions: &mut BindGroupEntryActions,
) {
    ui.add(
        Label::new(bind_group_entry_title(entry))
            .selectable(false)
            .sense(Sense::click()),
    )
    .context_menu(|ui| match entry {
        UnifiedEntry::Static { .. } => {
            if ui.button("Delete Bind Group").clicked() {
                actions.delete_index = Some(index);
                ui.close();
            }
        }
        UnifiedEntry::Material { .. } => {
            if ui.button("Remove Material Bind Group").clicked() {
                actions.remove_material = true;
                ui.close();
            }
        }
    });
}

fn bind_group_entry_title(entry: &UnifiedEntry) -> String {
    match entry {
        UnifiedEntry::Static { slot, .. } => format!("Slot {}", slot),
        UnifiedEntry::Material { slot } => format!("Slot {} (Material)", slot),
    }
}

fn bind_group_entry_fields_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    pipeline_id: usize,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    entry: &mut UnifiedEntry,
) {
    Grid::new((
        "pipeline_bind_groups_grid",
        render_pass_id,
        pipeline_id,
        index,
    ))
    .num_columns(2)
    .show(ui, |ui| match entry {
        UnifiedEntry::Static { slot, bg_id } => {
            static_bind_group_fields_ui(ui, pipeline_id, index, bind_groups, slot, bg_id);
        }
        UnifiedEntry::Material { slot } => {
            material_bind_group_fields_ui(ui, slot);
        }
    });
}

fn static_bind_group_fields_ui(
    ui: &mut egui::Ui,
    pipeline_id: usize,
    index: usize,
    bind_groups: &Storage<BindGroup>,
    slot: &mut u32,
    bg_id: &mut Option<BindGroupId>,
) {
    ui.label("Slot");
    ui.add(egui::DragValue::new(slot).speed(0.1));
    ui.end_row();

    let mut selected = *bg_id;
    ui.label("Bind Group");
    egui::ComboBox::from_id_salt(("static_bg", pipeline_id, index))
        .selected_text_storage_opt(bind_groups, selected)
        .show_ui_storage_opt_with_none(ui, bind_groups, &mut selected);
    ui.end_row();

    *bg_id = selected;
}

fn material_bind_group_fields_ui(ui: &mut egui::Ui, slot: &mut u32) {
    ui.label("Slot");
    ui.add(egui::DragValue::new(slot).speed(0.1));
    ui.end_row();

    ui.label("");
    ui.label(RichText::new("From RenderDraw::Model").italics());
    ui.end_row();
}

fn normalize_entry_slots(entries: &mut [UnifiedEntry]) {
    for (index, entry) in entries.iter_mut().enumerate() {
        match entry {
            UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => {
                *slot = index as u32;
            }
        }
    }
}

fn bind_group_entries_footer_ui(
    ui: &mut egui::Ui,
    pipeline: &RenderPipeline,
    entries: &mut Vec<UnifiedEntry>,
) {
    ui.add_space(6.0);

    let can_add_bind_group = entries.len() < MAX_RENDER_PASS_BIND_GROUPS;
    let max_bind_groups_message =
        format!("The maximum is {MAX_RENDER_PASS_BIND_GROUPS} bind groups.");

    if ui
        .add_enabled(can_add_bind_group, egui::Button::new("Add Bind Group"))
        .on_disabled_hover_text(max_bind_groups_message.clone())
        .clicked()
    {
        entries.push(UnifiedEntry::Static {
            slot: next_entry_slot(entries),
            bg_id: None,
        });
    }

    if let RenderDraw::Model {
        material_bind_group_slot: None,
        ..
    } = pipeline.draw
    {
        ui.add_space(4.0);
        if ui
            .add_enabled(
                can_add_bind_group,
                egui::Button::new("Add Material Bind Group"),
            )
            .on_disabled_hover_text(max_bind_groups_message)
            .on_hover_text("Create a material bind group entry tied to the model draw")
            .clicked()
        {
            entries.push(UnifiedEntry::Material {
                slot: next_entry_slot(entries),
            });
        }
    }
}

fn next_entry_slot(entries: &[UnifiedEntry]) -> u32 {
    first_available_slot(entries.iter().map(|entry| match entry {
        UnifiedEntry::Static { slot, .. } | UnifiedEntry::Material { slot } => *slot,
    }))
}

fn apply_bind_group_entry_actions(entries: &mut Vec<UnifiedEntry>, actions: BindGroupEntryActions) {
    if let Some(index) = actions.delete_index
        && index < entries.len()
    {
        entries.remove(index);
    }

    if actions.remove_material {
        entries.retain(|entry| !matches!(entry, UnifiedEntry::Material { .. }));
    }
}

fn bind_group_entries_hint_ui(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.add(hint(|ui| {
        ui.label("Right-click a");
        ui.label(RichText::new("Slot").strong());
        ui.label("to delete it, or drag to reorder.");
    }));
}

fn split_bind_group_entries(
    entries: &[UnifiedEntry],
) -> (Vec<(u32, Option<BindGroupId>)>, Option<u32>) {
    let static_bind_groups = entries
        .iter()
        .filter_map(|entry| match entry {
            UnifiedEntry::Static { slot, bg_id } => Some((*slot, *bg_id)),
            UnifiedEntry::Material { .. } => None,
        })
        .collect();

    let material_bind_group_slot = entries.iter().find_map(|entry| match entry {
        UnifiedEntry::Material { slot } => Some(*slot),
        UnifiedEntry::Static { .. } => None,
    });

    (static_bind_groups, material_bind_group_slot)
}

fn apply_material_bind_group_slot(
    pipeline: &mut RenderPipeline,
    draw_before: RenderDraw,
    new_material_slot: Option<u32>,
) {
    if let RenderDraw::Model {
        material_bind_group_slot,
        ..
    } = &pipeline.draw
        && *material_bind_group_slot != new_material_slot
    {
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
                    draw_kind_fields_ui(ui, pipeline, &mut kind, kind_before, &mut edited);
                    draw_fields_ui(ui, pipeline.id, models, &mut edited);
                });
        });

    if edited != before {
        pipeline.set_draw(edited);
    }
}

fn draw_kind_fields_ui(
    ui: &mut egui::Ui,
    pipeline: &RenderPipeline,
    kind: &mut DrawKind,
    kind_before: DrawKind,
    edited: &mut RenderDraw,
) {
    ui.label("Draw Kind");
    combo_list(ui, ("draw_kind", pipeline.id), DrawKind::iter(), kind);
    ui.end_row();

    if *kind != kind_before {
        *edited = draw_default(*kind);
        if let RenderDraw::Model {
            material_bind_group_slot,
            ..
        } = edited
        {
            *material_bind_group_slot = Some(first_available_slot(
                pipeline.static_bind_groups.iter().map(|(slot, _)| *slot),
            ));
        }
    }
}

fn draw_fields_ui(
    ui: &mut egui::Ui,
    pipeline_id: usize,
    models: &Storage<Model>,
    edited: &mut RenderDraw,
) {
    match edited {
        RenderDraw::Model {
            model_id,
            instances,
            mesh_vertex_slot,
            material_bind_group_slot: _,
        } => model_draw_fields_ui(
            ui,
            pipeline_id,
            models,
            model_id,
            instances,
            mesh_vertex_slot,
        ),
        RenderDraw::Direct {
            vertices,
            instances,
        } => direct_draw_fields_ui(ui, vertices, instances),
    }
}

fn model_draw_fields_ui(
    ui: &mut egui::Ui,
    pipeline_id: usize,
    models: &Storage<Model>,
    model_id: &mut Option<crate::project::ModelId>,
    instances: &mut Range<u32>,
    mesh_vertex_slot: &mut u32,
) {
    ui.label("Model");
    egui::ComboBox::from_id_salt(("draw_model_id", pipeline_id))
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

fn direct_draw_fields_ui(ui: &mut egui::Ui, vertices: &mut Range<u32>, instances: &mut Range<u32>) {
    ui.label("Vertices");
    range_u32_edit(ui, vertices);
    ui.end_row();

    ui.label("Instances");
    range_u32_edit(ui, instances);
    ui.end_row();
}
