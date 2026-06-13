use egui::RichText;

use crate::{
    project::{
        BindGroupId, ComputePassId,
        resource::{
            bindgroup::BindGroup,
            compute_pass::{ComputePass, WorkGroups},
            shader::Shader,
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector,
            selector::ComboBoxExt,
        },
        pane::StateSnapshot,
    },
    utils::shader_preview::ShaderGenCtx,
};

impl StateSnapshot<'_> {
    pub fn compute_pass_inspector_ui(&mut self, ui: &mut egui::Ui, compute_pass_id: ComputePassId) {
        let Ok(compute_pass) = self.project.compute_passes.get_mut(compute_pass_id) else {
            ui.label("Compute Pass couldn't be found.");
            return;
        };

        compute_pass_fields_ui(ui, compute_pass, &self.project.shaders);

        compute_pass_bind_groups_ui(ui, compute_pass_id, compute_pass, &self.project.bind_groups);

        if let Ok(pass) = self.project.compute_passes.get(compute_pass_id) {
            let ctx = ShaderGenCtx::from_project(self.project);
            shader_code_section(ui, pass, &ctx);
        }
    }
}

fn compute_pass_fields_ui(
    ui: &mut egui::Ui,
    compute_pass: &mut ComputePass,
    shaders: &Storage<Shader>,
) {
    inspector::section(ui, "Settings", |ui| {
        inspector::field_grid(ui, "compute_pass_inspector_grid", |ui| {
            let mut shader_id = compute_pass.shader();
            if inspector::storage_opt_combo_row(
                ui,
                "Shader",
                "compute_pass_shader",
                shaders,
                &mut shader_id,
            ) {
                compute_pass.set_shader(shader_id);
            }

            let (mut x, mut y, mut z) = compute_pass.work_groups().into();

            inspector::u32_drag_row(ui, "Workgroups X", &mut x, 1..=u32::MAX);
            inspector::u32_drag_row(ui, "Workgroups Y", &mut y, 1..=u32::MAX);
            inspector::u32_drag_row(ui, "Workgroups Z", &mut z, 1..=u32::MAX);

            compute_pass.set_work_groups(WorkGroups::new(x, y, z));
        });
    });
}

fn compute_pass_bind_groups_ui(
    ui: &mut egui::Ui,
    compute_pass_id: ComputePassId,
    compute_pass: &mut ComputePass,
    bind_groups: &Storage<BindGroup>,
) {
    let before = compute_pass.bind_groups().to_vec();
    let mut entries = before.clone();

    inspector::section(ui, &format!("Bind Groups ({})", entries.len()), |ui| {
        if entries.is_empty() {
            ui.label("No bind groups in compute pass.");
        }

        let mut edits = draggable_list(
            ui,
            (compute_pass_id, "compute_pass_bind_groups"),
            &entries,
            |ui, bind_group_id, index, handle, edits| {
                compute_pass_bind_group_row_ui(
                    ui,
                    handle,
                    bind_groups,
                    index,
                    *bind_group_id,
                    edits,
                );
            },
        );

        ui.add_space(6.0);

        if ui.button("Add Bind Group").clicked() {
            edits.push_add_edit(None);
        }

        if !entries.is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click a");
                ui.label(RichText::new("Bind Group").strong());
                ui.label("to delete it, or drag it to reorder.");
            }));
        }

        edits.apply(&mut entries);

        if entries != before {
            compute_pass.set_bind_groups(entries);
        }
    });
}

fn compute_pass_bind_group_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    bind_group_id: Option<BindGroupId>,
    edits: &mut ListEdits<Option<BindGroupId>>,
) {
    handle.ui(ui, |ui| {
        ui.add(egui::Label::new(format!("Slot {index}")).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Remove Bind Group").clicked() {
                    edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    ui.indent("entry", |ui| {
        let mut selected_bind_group = bind_group_id;

        // TODO: use something inside inspector.rs
        egui::ComboBox::from_id_salt(("compute_pass_bind_group", index))
            .selected_text_storage_opt(bind_groups, selected_bind_group)
            .show_ui_storage_opt_with_none(ui, bind_groups, &mut selected_bind_group);

        if selected_bind_group != bind_group_id {
            edits.push_set_edit(index, selected_bind_group);
        }
    });
}
