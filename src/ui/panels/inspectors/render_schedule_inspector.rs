use egui::RichText;

use crate::{
    project::RenderScheduleId,
    state::StateEvent,
    ui::{
        components::{hint::hint, selector::ComboBoxExt},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn render_schedule_inspector_ui(&mut self, ui: &mut egui::Ui) {
        let entries = self.project.render_schedule.entries();

        if entries.is_empty() {
            ui.label("No render passes in the render schedule.");
        }

        let response = egui_dnd::dnd(ui, RenderScheduleId).show_custom(|ui, iter| {
            for (index, entry) in entries.iter().copied().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                ui.push_id(index, |ui| {
                    let item_id = egui::Id::new(entry.id());
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
                                            self.pending_events
                                                .push(StateEvent::DeleteRenderScheduleEntry(index));
                                            ui.close();
                                        }
                                    });
                                });
                            });

                            ui.indent("entry", |ui| {
                                egui::Grid::new(("render_schedule_entry", index))
                                    .num_columns(2)
                                    .spacing([8.0, 4.0])
                                    .show(ui, |ui| {
                                        let before = entry.render_pass_id();
                                        let mut id = before;

                                        let render_passes = &self.project.render_passes;

                                        ui.label("Render Pass");
                                        egui::ComboBox::from_id_salt("render_schedule_pass")
                                            .selected_text_storage_opt(render_passes, id)
                                            .show_ui_storage_opt(ui, render_passes, &mut id);

                                        ui.end_row();

                                        if id != before {
                                            self.pending_events.push(
                                                StateEvent::UpdateRenderScheduleEntry(index, id),
                                            );
                                        }
                                    });
                            });
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            self.pending_events
                .push(StateEvent::ReorderRenderScheduleEntry(update));
        }

        ui.add_space(6.0);

        if ui.button("Add Render Pass").clicked() {
            self.pending_events
                .push(StateEvent::CreateRenderScheduleEntry);
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
