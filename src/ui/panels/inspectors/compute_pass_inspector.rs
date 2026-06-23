use crate::{
    project::{
        BindGroupId, ComputePassId,
        resource::{
            bindgroup::BindGroup,
            compute_pass::{ComputePass, DispatchPolicy, WorkGroups},
            shader::Shader,
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            draggable_list::{ListEdits, draggable_list},
            field,
            field_docs::field_doc,
            inspector, resource_icons,
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
        field::field_grid(ui, "compute_pass_inspector_grid", |ui| {
            let mut shader_id = compute_pass.shader();
            if field::row_doc(
                ui,
                "Shader",
                field_doc!(
                    "The compute shader run by this pass.\n\n\
                    WGSL marks the entry point with `@compute`; GLSL uses `void main()` in a \
                    `.comp` file.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpuprogrammablestage)"
                ),
                |ui| inspector::storage_combo(ui, "compute_pass_shader", shaders, &mut shader_id),
            ) {
                compute_pass.set_shader(shader_id);
            }

            let (mut x, mut y, mut z) = compute_pass.work_groups().into();

            inspector::u32_drag_row_doc(
                ui,
                "Workgroups X",
                field_doc!(
                    "Number of workgroups dispatched along the **X** axis.\n\n\
                    The shader runs once per workgroup; the total invocations are this count \
                    multiplied by the `@workgroup_size` declared in the shader.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpucomputepassencoder-dispatchworkgroups)"
                ),
                &mut x,
                1..=u32::MAX,
            );
            inspector::u32_drag_row_doc(
                ui,
                "Workgroups Y",
                field_doc!("Number of workgroups dispatched along the **Y** axis."),
                &mut y,
                1..=u32::MAX,
            );
            inspector::u32_drag_row_doc(
                ui,
                "Workgroups Z",
                field_doc!("Number of workgroups dispatched along the **Z** axis."),
                &mut z,
                1..=u32::MAX,
            );

            compute_pass.set_work_groups(WorkGroups::new(x, y, z));

            compute_pass_dispatch_ui(ui, compute_pass);
        });
    });
}

fn dispatch_label(policy: &DispatchPolicy) -> &'static str {
    match policy {
        DispatchPolicy::OnChange => "On Change",
        DispatchPolicy::EveryFrame => "Every Frame",
        DispatchPolicy::Periodic { .. } => "Periodic",
    }
}

fn compute_pass_dispatch_ui(ui: &mut egui::Ui, compute_pass: &mut ComputePass) {
    let mut policy = compute_pass.dispatch();
    let mut changed = false;

    changed |= field::row_doc(
        ui,
        "Dispatch",
        field_doc!(
            "When this pass re-dispatches.\n\n\
            **On Change** runs only when an input changes (good for one-shot bakes). \
            **Every Frame** runs once per rendered frame. \
            **Periodic** runs at a fixed cadence independent of the framerate.\n\n\
            Make sure to also add this pass to the presentation's compute pass list, or it won't run at all."
        ),
        |ui| {
            let before = std::mem::discriminant(&policy);
            egui::ComboBox::from_id_salt("compute_pass_dispatch")
                .selected_text(dispatch_label(&policy))
                .show_ui(ui, |ui| {
                    dispatch_option(ui, &mut policy, DispatchPolicy::OnChange);
                    dispatch_option(ui, &mut policy, DispatchPolicy::EveryFrame);
                    // Keep the existing interval if already Periodic; otherwise
                    // seed a sensible default when switching in.
                    let is_periodic = matches!(policy, DispatchPolicy::Periodic { .. });
                    if ui.selectable_label(is_periodic, "Periodic").clicked() && !is_periodic {
                        policy = DispatchPolicy::Periodic {
                            interval: instant::Duration::from_millis(50),
                        };
                    }
                });
            std::mem::discriminant(&policy) != before
        },
    );

    if let DispatchPolicy::Periodic { interval } = &mut policy {
        let mut secs = interval.as_secs_f32();
        if inspector::f32_drag_row_doc(
            ui,
            "Interval (s)",
            field_doc!(
                "Seconds between dispatches. The pass runs once each interval of \
                accumulated frame time, so the rate is the same on any monitor."
            ),
            &mut secs,
            0.0001..=10.0,
            0.001,
            4,
        ) {
            *interval = instant::Duration::from_secs_f32(secs.max(0.0001));
            changed = true;
        }
    }

    if changed {
        compute_pass.set_dispatch(policy);
    }
}

fn dispatch_option(ui: &mut egui::Ui, policy: &mut DispatchPolicy, option: DispatchPolicy) {
    let selected = std::mem::discriminant(policy) == std::mem::discriminant(&option);
    if ui
        .selectable_label(selected, dispatch_label(&option))
        .clicked()
    {
        *policy = option;
    }
}

fn compute_pass_bind_groups_ui(
    ui: &mut egui::Ui,
    compute_pass_id: ComputePassId,
    compute_pass: &mut ComputePass,
    bind_groups: &Storage<BindGroup>,
) {
    let before = compute_pass.bind_groups().to_vec();
    let mut entries = before.clone();

    inspector::section_doc(
        ui,
        &format!("Bind Groups ({})", entries.len()),
        field_doc!(
            "The Bind Groups bound while this pass runs, one per slot.\n\n\
            Slot order maps to `@group(n)` in the compute shader (top to bottom: group 0, 1, \
            and so on).\n\n\
            Drag to reorder, right-click to remove.\n\n\
            [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpucomputepassencoder-setbindgroup)"
        ),
        |ui| {
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

            inspector::add_from_storage_menu(
                ui,
                "Add Bind Group",
                bind_groups,
                "No bind groups.",
                |id| edits.push_add_edit(id),
            );

            edits.apply(&mut entries);

            if entries != before {
                compute_pass.set_bind_groups(entries);
            }
        },
    );
}

fn compute_pass_bind_group_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    bind_groups: &Storage<BindGroup>,
    index: usize,
    bind_group_id: BindGroupId,
    edits: &mut ListEdits<BindGroupId>,
) {
    handle.ui(ui, |ui| {
        let label = resource_icons::drag_handle_text(ui, &format!("Slot {index}"));
        ui.add(egui::Label::new(label).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Remove Bind Group").clicked() {
                    edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    ui.indent("entry", |ui| {
        let mut selected_bind_group = bind_group_id;

        inspector::storage_id_combo(
            ui,
            ("compute_pass_bind_group", index),
            bind_groups,
            &mut selected_bind_group,
        );

        if selected_bind_group != bind_group_id {
            edits.push_set_edit(index, selected_bind_group);
        }
    });
}
