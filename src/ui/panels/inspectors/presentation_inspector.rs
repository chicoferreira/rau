use crate::{
    project::{
        ComputePassId, RenderPassId,
        resource::{compute_pass::ComputePass, render_pass::RenderPass},
        storage::Storage,
    },
    ui::{
        components::{
            draggable_list::{ListEdits, draggable_list},
            field,
            field_docs::field_doc,
            inspector, resource_icons,
        },
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn presentation_inspector_ui(&mut self, ui: &mut egui::Ui) {
        inspector::section(ui, "Main Viewport", |ui| {
            field::field_grid(ui, "presentation_inspector_grid", |ui| {
                let mut id = self.project.presentation.main_viewport();
                if field::row_doc(
                    ui,
                    "Main Viewport",
                    field_doc!(
                        "The Viewport presented when the project opens.\n\n\
                        Optional. Leave as **None** to present nothing."
                    ),
                    |ui| {
                        inspector::storage_opt_combo(
                            ui,
                            "presentation_main_viewport",
                            &self.project.viewports,
                            &mut id,
                        )
                    },
                ) {
                    self.project.presentation.set_main_viewport(id);
                }
            });
        });

        inspector::section_doc(
            ui,
            "Compute Passes",
            field_doc!(
                "The ordered **compute** passes that can be dispatched each frame, before the render \
                passes. Order here is execution order.\n\n\
                Each pass's dispatch policy (On Change / Every Frame / Periodic) decides whether \
                it actually runs on a given frame.\n\n\
                Drag to reorder, right-click to remove."
            ),
            |ui| {
                presentation_compute_pass_list_ui(ui, self);
            },
        );

        inspector::section_doc(
            ui,
            "Render Passes",
            field_doc!(
                "The ordered list of **steps** run every frame to produce the presentation.\n\n\
                Drag to reorder, right-click to remove."
            ),
            |ui| {
                presentation_render_pass_list_ui(ui, self);
            },
        );
    }
}

fn presentation_compute_pass_list_ui(ui: &mut egui::Ui, state: &mut StateSnapshot<'_>) {
    let before = state.project.presentation.compute_passes().to_vec();
    let mut compute_passes = before.clone();

    if compute_passes.is_empty() {
        ui.label("No compute passes in the presentation.");
    }

    let compute_pass_storage = &state.project.compute_passes;

    let mut edits = draggable_list(
        ui,
        "presentation_compute_pass_list",
        &compute_passes,
        |ui, compute_pass_id, index, handle, edits| {
            presentation_compute_pass_row_ui(
                ui,
                handle,
                index,
                *compute_pass_id,
                compute_pass_storage,
                edits,
            );
        },
    );

    ui.add_space(6.0);

    inspector::add_from_storage_menu(
        ui,
        "Add Compute Pass",
        compute_pass_storage,
        "No compute passes.",
        |id| edits.push_add_edit(id),
    );

    edits.apply(&mut compute_passes);

    if compute_passes != before {
        state
            .project
            .presentation
            .set_compute_passes(compute_passes);
    }
}

fn presentation_compute_pass_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    compute_pass_id: ComputePassId,
    compute_passes: &Storage<ComputePass>,
    edits: &mut ListEdits<ComputePassId>,
) {
    handle.ui(ui, |ui| {
        let label = resource_icons::drag_handle_text(ui, &format!("Step {}", index + 1));
        ui.add(egui::Label::new(label).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Remove Compute Pass").clicked() {
                    edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    let mut selected = compute_pass_id;

    ui.indent(("presentation_compute_pass_select", index), |ui| {
        inspector::storage_id_combo(
            ui,
            ("presentation_compute_pass_select", index),
            compute_passes,
            &mut selected,
        );
    });

    if selected != compute_pass_id {
        edits.push_set_edit(index, selected);
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
        let label = resource_icons::drag_handle_text(ui, &format!("Step {}", index + 1));
        ui.add(egui::Label::new(label).sense(egui::Sense::click()))
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
