use egui::RichText;

use crate::{
    project::FramePlanId,
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
            .map(|entry| (entry.id(), entry.render_pass_id()))
            .collect::<Vec<_>>();
        let render_passes = &self.project.render_passes;
        let mut entry_edits = Vec::new();

        if entries.is_empty() {
            ui.label("No render passes in the frame plan.");
        }

        let response = egui_dnd::dnd(ui, FramePlanId).show_custom(|ui, iter| {
            for (index, (entry_id, render_pass_id)) in entries.iter().copied().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                ui.push_id(index, |ui| {
                    let item_id = egui::Id::new(entry_id);
                    iter.next(ui, item_id, index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            handle.ui(ui, |ui| {
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
                            });

                            ui.indent("entry", |ui| {
                                egui::Grid::new(("frame_plan_step", index))
                                    .num_columns(2)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        let before = render_pass_id;
                                        let mut id = before;

                                        ui.label("Render Pass");
                                        egui::ComboBox::from_id_salt("frame_plan_pass")
                                            .selected_text_storage_opt(render_passes, id)
                                            .show_ui_storage_opt(ui, render_passes, &mut id);

                                        ui.end_row();

                                        if id != before {
                                            entry_edits.push(FramePlanEdit::Update(index, id));
                                        }
                                    });
                            });
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            entry_edits.push(FramePlanEdit::Reorder(update));
        }

        ui.add_space(6.0);

        if ui.button("Add Render Pass").clicked() {
            self.project.frame_plan.add(None);
        }

        for edit in entry_edits {
            match edit {
                FramePlanEdit::Delete(index) => self.project.frame_plan.remove_entry(index),
                FramePlanEdit::Update(index, render_pass_id) => {
                    self.project.frame_plan.update_entry(index, render_pass_id);
                }
                FramePlanEdit::Reorder(update) => {
                    self.project
                        .frame_plan
                        .reorder_entries(update.from, update.to);
                }
            }
        }

        if !entries.is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click an");
                ui.label(RichText::new("Entry").strong());
                ui.label("to remove it or drag it to reorder it.");
            }));
        }
    }
}

enum FramePlanEdit {
    Delete(usize),
    Update(usize, Option<crate::project::RenderPassId>),
    Reorder(egui_dnd::DragUpdate),
}
