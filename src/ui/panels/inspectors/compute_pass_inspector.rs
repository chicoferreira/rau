use egui::{CollapsingHeader, Grid, RichText};

use crate::{
    project::{
        BindGroupId, ComputePassId, bindgroup::BindGroup, compute_pass::ComputePass,
        storage::Storage,
    },
    ui::{
        components::{hint::hint, selector::ComboBoxExt},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn compute_pass_inspector_ui(&mut self, ui: &mut egui::Ui, compute_pass_id: ComputePassId) {
        let Ok(compute_pass) = self.project.compute_passes.get_mut(compute_pass_id) else {
            ui.label("Compute Pass couldn't be found.");
            return;
        };

        Grid::new("compute_pass_inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                let shaders = &self.project.shaders;

                ui.label("Shader");
                let mut shader_id = compute_pass.shader();
                egui::ComboBox::from_id_salt("compute_pass_shader")
                    .selected_text_storage_opt(shaders, shader_id)
                    .show_ui_storage_opt_with_none(ui, shaders, &mut shader_id);
                ui.end_row();

                if shader_id != compute_pass.shader() {
                    compute_pass.set_shader(shader_id);
                }

                let (mut work_groups_x, mut work_groups_y, mut work_groups_z) =
                    compute_pass.work_groups();

                ui.label("Workgroups X");
                ui.add(
                    egui::DragValue::new(&mut work_groups_x)
                        .range(1..=u32::MAX)
                        .speed(1),
                );
                ui.end_row();

                ui.label("Workgroups Y");
                ui.add(
                    egui::DragValue::new(&mut work_groups_y)
                        .range(1..=u32::MAX)
                        .speed(1),
                );
                ui.end_row();

                ui.label("Workgroups Z");
                ui.add(
                    egui::DragValue::new(&mut work_groups_z)
                        .range(1..=u32::MAX)
                        .speed(1),
                );
                ui.end_row();

                if (work_groups_x, work_groups_y, work_groups_z) != compute_pass.work_groups() {
                    compute_pass.set_work_groups(work_groups_x, work_groups_y, work_groups_z);
                }
            });

        ui.add_space(4.0);

        let bind_group_count = compute_pass.bind_groups().len();

        CollapsingHeader::new(format!("Bind Groups ({bind_group_count})"))
            .default_open(true)
            .show(ui, |ui| {
                if compute_pass.bind_groups().is_empty() {
                    ui.label("No bind groups in compute pass.");
                }

                let bind_groups = &self.project.bind_groups;
                compute_pass_bind_group_list_ui(ui, compute_pass_id, compute_pass, bind_groups);

                ui.add_space(6.0);

                if ui.button("Add Bind Group").clicked() {
                    compute_pass.add_bind_group(None);
                }

                if bind_group_count > 0 {
                    ui.add_space(6.0);
                    ui.add(hint(|ui| {
                        ui.label("Right-click a");
                        ui.label(RichText::new("Bind Group").strong());
                        ui.label("to delete it, or drag it to reorder.");
                    }));
                }
            });
    }
}

fn compute_pass_bind_group_list_ui(
    ui: &mut egui::Ui,
    compute_pass_id: ComputePassId,
    compute_pass: &mut ComputePass,
    bind_groups: &Storage<BindGroup>,
) {
    let bind_group_slots = compute_pass.bind_groups().to_vec();

    let response =
        egui_dnd::dnd(ui, (compute_pass_id, "compute_pass_bind_groups")).show_custom(|ui, iter| {
            for (index, entry) in bind_group_slots.iter().copied().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }

                ui.push_id(entry.id(), |ui| {
                    let item_id = egui::Id::new(entry.id());
                    iter.next(ui, item_id, index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            compute_pass_bind_group_row_ui(
                                ui,
                                handle,
                                compute_pass,
                                bind_groups,
                                index,
                                entry.bind_group_id(),
                            );
                        })
                    });
                });
            }
        });

    if let Some(update) = response.final_update() {
        compute_pass.reorder_bind_groups(update.from, update.to);
    }
}

fn compute_pass_bind_group_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    compute_pass: &mut ComputePass,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    bind_group_id: Option<BindGroupId>,
) {
    handle.ui(ui, |ui| {
        compute_pass_bind_group_title_ui(ui, compute_pass, index);

        ui.indent("entry", |ui| {
            let mut selected_bind_group = bind_group_id;
            bind_group_slot_ui(ui, bind_groups, index, &mut selected_bind_group);

            if selected_bind_group != bind_group_id {
                compute_pass.set_bind_group(index, selected_bind_group);
            }
        });
    });
}

fn compute_pass_bind_group_title_ui(
    ui: &mut egui::Ui,
    compute_pass: &mut ComputePass,
    index: usize,
) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(format!("Bind Group {index}"))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Delete Bind Group").clicked() {
                compute_pass.remove_bind_group(index);
                ui.close();
            }
        });
    });
}

fn bind_group_slot_ui(
    ui: &mut egui::Ui,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    bind_group_id: &mut Option<BindGroupId>,
) {
    Grid::new(("compute_pass_bind_group_grid", index))
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Bind Group");
            egui::ComboBox::from_id_salt(("compute_pass_bind_group", index))
                .selected_text_storage_opt(bind_groups, *bind_group_id)
                .show_ui_storage_opt_with_none(ui, bind_groups, bind_group_id);
            ui.end_row();
        });
}
