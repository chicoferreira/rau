use egui::RichText;

use crate::{
    project::{ProjectResource, RenderPassId, resource::render_pass::RenderPass, storage::Storage},
    ui::{
        components::{
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector,
        },
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn presentation_inspector_ui(&mut self, ui: &mut egui::Ui) {
        inspector::field_grid(ui, "presentation_inspector_grid", |ui| {
            let mut id = self.project.presentation.main_viewport();
            if inspector::storage_opt_combo_row(
                ui,
                "Main Viewport",
                "presentation_main_viewport",
                &self.project.viewports,
                &mut id,
            ) {
                self.project.presentation.set_main_viewport(id);
            }
        });

        ui.add_space(4.0);

        presentation_render_pass_list_ui(ui, self);
    }
}

fn presentation_render_pass_list_ui(ui: &mut egui::Ui, state: &mut StateSnapshot<'_>) {
    let before = state.project.presentation.render_passes().to_vec();
    let mut render_passes = before.clone();

    if render_passes.is_empty() {
        ui.label("No render passes in the presentation.");
    }

    let render_pass_storage = &state.project.render_passes;

    let mut edits = draggable_list(
        ui,
        "presentation_render_pass_list",
        &render_passes,
        |ui, render_pass_id, index, handle, edits| {
            presentation_render_pass_row_ui(
                ui,
                handle,
                index,
                *render_pass_id,
                render_pass_storage,
                edits,
            );
        },
    );

    ui.add_space(6.0);

    ui.menu_button("Add Render Pass", |ui| {
        let mut has_render_passes = false;
        for (id, render_pass) in render_pass_storage.list_sorted() {
            has_render_passes = true;
            if ui.button(render_pass.label()).clicked() {
                edits.push_add_edit(id);
                ui.close();
            }
        }

        if !has_render_passes {
            ui.label("No render passes.");
        }
    });

    if !render_passes.is_empty() {
        ui.add_space(6.0);
        ui.add(hint(|ui| {
            ui.label("Right-click a");
            ui.label(RichText::new("Render Pass").strong());
            ui.label("to remove it, or drag to reorder.");
        }));
    }

    edits.apply(&mut render_passes);

    if render_passes != before {
        state.project.presentation.set_render_passes(render_passes);
    }
}

fn presentation_render_pass_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    render_pass_id: RenderPassId,
    render_passes: &Storage<RenderPass>,
    edits: &mut ListEdits<RenderPassId>,
) {
    handle.ui(ui, |ui| {
        ui.add(
            egui::Label::new(format!("Step {}", index + 1))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Remove Render Pass").clicked() {
                edits.push_remove_edit(index);
                ui.close();
            }
        });
    });

    let mut selected = render_pass_id;

    ui.indent(("presentation_render_pass_select", index), |ui| {
        // TODO: replace with one of the combobox components
        egui::ComboBox::from_id_salt(("presentation_render_pass_select", index))
            .selected_text(render_pass_label(render_passes, selected))
            .show_ui(ui, |ui| {
                for (id, render_pass) in render_passes.list_sorted() {
                    ui.selectable_value(&mut selected, id, render_pass.label());
                }
            });
    });

    if selected != render_pass_id {
        edits.push_set_edit(index, selected);
    }
}

fn render_pass_label(render_passes: &Storage<RenderPass>, render_pass_id: RenderPassId) -> String {
    render_passes
        .get_label(render_pass_id)
        .map(str::to_owned)
        .unwrap_or_else(|_| format!("Unknown {render_pass_id:?}"))
}
