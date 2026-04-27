use egui::RichText;

use crate::{
    project::{
        FramePlanId, RenderPassId,
        resource::{frame_plan::FramePlanStepId, render_pass::RenderPass},
        storage::Storage,
    },
    ui::{
        components::{hint::hint, selector::ComboBoxExt},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn frame_plan_inspector_ui(&mut self, ui: &mut egui::Ui) {
        let entries = self
            .project
            .frame_plan
            .entries()
            .iter()
            .map(|entry| FramePlanEntrySnapshot {
                id: entry.id(),
                render_pass_id: entry.render_pass_id(),
            })
            .collect::<Vec<_>>();
        let render_passes = &self.project.render_passes;
        let mut entry_edits = Vec::new();

        if entries.is_empty() {
            ui.label("No render passes in the frame plan.");
        }

        let response = egui_dnd::dnd(ui, FramePlanId).show_custom(|ui, iter| {
            for (index, entry) in entries.iter().copied().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                ui.push_id(index, |ui| {
                    let item_id = egui::Id::new(entry.id);
                    iter.next(ui, item_id, index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            frame_plan_entry_ui(
                                ui,
                                handle,
                                index,
                                entry.render_pass_id,
                                render_passes,
                                &mut entry_edits,
                            );
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            entry_edits.push(FramePlanEdit::Reorder(update));
        }

        if frame_plan_add_button_ui(ui) {
            self.project.frame_plan.add(None);
        }

        apply_frame_plan_edits(self, entry_edits);

        if !entries.is_empty() {
            frame_plan_hint_ui(ui);
        }
    }
}

#[derive(Clone, Copy)]
struct FramePlanEntrySnapshot {
    id: FramePlanStepId,
    render_pass_id: Option<RenderPassId>,
}

fn frame_plan_entry_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    render_pass_id: Option<RenderPassId>,
    render_passes: &Storage<RenderPass>,
    entry_edits: &mut Vec<FramePlanEdit>,
) {
    handle.ui(ui, |ui| {
        frame_plan_entry_title_ui(ui, index, entry_edits);

        ui.indent("entry", |ui| {
            frame_plan_entry_fields_ui(ui, index, render_pass_id, render_passes, entry_edits);
        });
    });
}

fn frame_plan_entry_title_ui(
    ui: &mut egui::Ui,
    index: usize,
    entry_edits: &mut Vec<FramePlanEdit>,
) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(format!("Step {}", index + 1))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Delete Entry").clicked() {
                entry_edits.push(FramePlanEdit::Delete(index));
                ui.close();
            }
        });
    });
}

fn frame_plan_entry_fields_ui(
    ui: &mut egui::Ui,
    index: usize,
    render_pass_id: Option<RenderPassId>,
    render_passes: &Storage<RenderPass>,
    entry_edits: &mut Vec<FramePlanEdit>,
) {
    egui::Grid::new(("frame_plan_step", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            let before = render_pass_id;
            let mut id = before;

            ui.label("Render Pass");
            egui::ComboBox::from_id_salt("frame_plan_pass")
                .selected_text_storage_opt(render_passes, id)
                .show_ui_storage_opt_with_none(ui, render_passes, &mut id);
            ui.end_row();

            if id != before {
                entry_edits.push(FramePlanEdit::Update(index, id));
            }
        });
}

fn frame_plan_add_button_ui(ui: &mut egui::Ui) -> bool {
    ui.add_space(6.0);
    ui.button("Add Render Pass").clicked()
}

fn frame_plan_hint_ui(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.add(hint(|ui| {
        ui.label("Right-click an");
        ui.label(RichText::new("Entry").strong());
        ui.label("to remove it or drag it to reorder it.");
    }));
}

fn apply_frame_plan_edits(state: &mut StateSnapshot<'_>, entry_edits: Vec<FramePlanEdit>) {
    for edit in entry_edits {
        match edit {
            FramePlanEdit::Delete(index) => state.project.frame_plan.remove_entry(index),
            FramePlanEdit::Update(index, render_pass_id) => {
                state.project.frame_plan.update_entry(index, render_pass_id);
            }
            FramePlanEdit::Reorder(update) => {
                state
                    .project
                    .frame_plan
                    .reorder_entries(update.from, update.to);
            }
        }
    }
}

enum FramePlanEdit {
    Delete(usize),
    Update(usize, Option<RenderPassId>),
    Reorder(egui_dnd::DragUpdate),
}
