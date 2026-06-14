use egui::RichText;

use crate::{
    project::{RenderPassId, resource::render_pass::RenderPass, storage::Storage},
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
        inspector::section(ui, "Main Viewport", |ui| {
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
        });

        inspector::section(ui, "Render Passes", |ui| {
            presentation_render_pass_list_ui(ui, self);
        });
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

    inspector::add_from_storage_menu(
        ui,
        "Add Render Pass",
        render_pass_storage,
        "No render passes.",
        |id| edits.push_add_edit(id),
    );

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
        ui.add(egui::Label::new(format!("Step {}", index + 1)).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Remove Render Pass").clicked() {
                    edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    let mut selected = render_pass_id;

    ui.indent(("presentation_render_pass_select", index), |ui| {
        inspector::storage_id_combo(
            ui,
            ("presentation_render_pass_select", index),
            render_passes,
            &mut selected,
        );
    });

    if selected != render_pass_id {
        edits.push_set_edit(index, selected);
    }
}
