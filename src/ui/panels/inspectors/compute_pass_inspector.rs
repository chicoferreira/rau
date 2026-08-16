use egui::Widget;

use crate::{
    project::{
        BindGroupId, ComputePassId,
        resource::{
            bindgroup::BindGroup,
            compute_pass::{ComputePass, DispatchPolicy, DispatchUnit, WorkSize},
            dimension::{Dimension, DimensionRef},
            shader::Shader,
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            dimension_ref::dimension_ref_edit,
            draggable_list::{ListEdits, draggable_list},
            field,
            field_docs::{FieldDoc, field_doc},
            inspector::{self, AsRichText},
            resource_icons,
        },
        pane::StateSnapshot,
    },
    utils::shader_preview::ShaderGenCtx,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum WorkSizeKind {
    Fixed,
    Dimension,
}

impl WorkSizeKind {
    fn from_work_size(work_size: &WorkSize) -> Self {
        match work_size {
            WorkSize::Fixed(_) => Self::Fixed,
            WorkSize::Dimension(_) => Self::Dimension,
        }
    }
}

impl AsRichText for WorkSizeKind {
    fn as_rich_text(&self) -> egui::RichText {
        match self {
            Self::Fixed => "Fixed",
            Self::Dimension => "Dimension",
        }
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DispatchUnitKind {
    Workgroup,
    Invocation,
}

impl DispatchUnitKind {
    fn from_unit(unit: &DispatchUnit) -> Self {
        match unit {
            DispatchUnit::Workgroup => Self::Workgroup,
            DispatchUnit::Invocation { .. } => Self::Invocation,
        }
    }
}

impl AsRichText for DispatchUnitKind {
    fn as_rich_text(&self) -> egui::RichText {
        match self {
            Self::Workgroup => "Workgroups",
            Self::Invocation => "Invocations",
        }
        .into()
    }
}

const WORK_SIZE_KINDS: [WorkSizeKind; 2] = [WorkSizeKind::Fixed, WorkSizeKind::Dimension];
const DISPATCH_UNITS: [DispatchUnitKind; 2] =
    [DispatchUnitKind::Workgroup, DispatchUnitKind::Invocation];

impl StateSnapshot<'_> {
    pub fn compute_pass_inspector_ui(&mut self, ui: &mut egui::Ui, compute_pass_id: ComputePassId) {
        let Ok(compute_pass) = self.project.compute_passes.get_mut(compute_pass_id) else {
            ui.label("Compute Pass couldn't be found.");
            return;
        };

        compute_pass_fields_ui(
            ui,
            compute_pass,
            &self.project.shaders,
            &self.project.dimensions,
        );

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
    dimensions: &Storage<Dimension>,
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

            compute_pass_dispatch_ui(ui, compute_pass);
        });
    });

    compute_pass_dispatch_size_ui(ui, compute_pass, dimensions);
}

fn compute_pass_dispatch_size_ui(
    ui: &mut egui::Ui,
    compute_pass: &mut ComputePass,
    dimensions: &Storage<Dimension>,
) {
    inspector::section_doc(
        ui,
        "Dispatch Size",
        field_doc!(
            "How many times this pass runs the shader.\n\n\
            A compute pass is launched by `dispatchWorkgroups(x, y, z)`, the WebGPU call that \
            runs a **grid of workgroups** on the GPU. Every workgroup runs a fixed block of \
            **invocations**, set by the `@workgroup_size` the shader declares. So the number of \
            invocations along an axis is the value sent to `dispatchWorkgroups` times the \
            workgroup size declared in the shader.\n\n\
            With `@workgroup_size(8, 8, 1)`, a dispatch of `(16, 16, 1)` runs 16x16 workgroups \
            of 64 (8x8x1) invocations each, for 128x128 invocations in total.\n\n\
            **Example**: to run the shader once per pixel of a 1920x1080 Dimension, with a \
            shader declaring `@workgroup_size(8, 8, 1)`:\n\
            - Set **Unit** to Invocations, and **Workgroup Size** to 8, 8, 1.\n\
            - Set **Size X** and **Size Y** to that Dimension's width and height, and \
            **Size Z** to 1.\n\
            - In the shader, read `@builtin(global_invocation_id)`: its `x` and `y` are the \
            coordinates of the pixel that invocation covers.\n\n\
            That resolves to `dispatchWorkgroups(240, 135, 1)`. Sourcing the sizes from the \
            Dimension rather than typing 1920 and 1080 keeps it correct if the Dimension \
            resizes.\n\n\
            [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpucomputepassencoder-dispatchworkgroups)"
        ),
        |ui| {
            let before = compute_pass.dispatch_size();
            let mut dispatch_size = before;

            field::field_grid(ui, "compute_pass_dispatch_size_grid", |ui| {
                let mut unit_kind = DispatchUnitKind::from_unit(&dispatch_size.unit);
                if inspector::combo_row_doc(
                    ui,
                    "Unit",
                    field_doc!(
                        "Which of the two counts the sizes below are.\n\n\
                         - **Workgroups**: passed to `dispatchWorkgroups` unchanged.\n\
                         - **Invocations**: divided by the workgroup size to get the workgroup \
                         counts. Useful when the work is one invocation per element: a pixel or \
                         a grid cell, for instance."
                    ),
                    "compute_pass_dispatch_unit",
                    DISPATCH_UNITS,
                    &mut unit_kind,
                ) {
                    dispatch_size.unit = match unit_kind {
                        DispatchUnitKind::Workgroup => DispatchUnit::Workgroup,
                        DispatchUnitKind::Invocation => DispatchUnit::Invocation {
                            workgroup_size: [1, 1, 1],
                        },
                    };
                }

                if let DispatchUnit::Invocation { workgroup_size } = &mut dispatch_size.unit {
                    field::row_doc(
                        ui,
                        "Workgroup Size",
                        field_doc!(
                            "The workgroup size the sizes below are divided by. Set it to the \
                             same value the shader declares in its `@workgroup_size`.\n\n\
                             Nothing checks the two against each other.\n\n\
                             If set **larger** than the shader declares, it dispatches too few \
                             workgroups and leaves part of the range unprocessed.\n\n\
                             If set **smaller**, it dispatches invocations past the range, \
                             which matters only if the shader acts on ids beyond it.\n\n\
                             [WGSL spec](https://www.w3.org/TR/WGSL/#compute-shader-workgroups)"
                        ),
                        |ui| {
                            ui.horizontal(|ui| {
                                for axis in workgroup_size.iter_mut() {
                                    egui::DragValue::new(axis)
                                        .speed(1)
                                        .range(1..=u32::MAX)
                                        .ui(ui);
                                }
                            });
                        },
                    );
                }

                work_size_row(
                    ui,
                    "Size X",
                    field_doc!(
                        "The extent of the dispatch along the **X** axis, in the unit selected \
                    above.\n\n\
                    - **Fixed**: a constant entered here.\n\
                    - **Dimension**: read from a Dimension resource."
                    ),
                    "compute_pass_size_x",
                    &mut dispatch_size.x,
                    dimensions,
                );
                work_size_row(
                    ui,
                    "Size Y",
                    field_doc!("The extent of the dispatch along the **Y** axis."),
                    "compute_pass_size_y",
                    &mut dispatch_size.y,
                    dimensions,
                );
                work_size_row(
                    ui,
                    "Size Z",
                    field_doc!(
                        "The extent of the dispatch along the **Z** axis.\n\n\
                    A Dimension carries only a width and a height, so it has no value that \
                    corresponds to this axis, so this is typically a fixed value."
                    ),
                    "compute_pass_size_z",
                    &mut dispatch_size.z,
                    dimensions,
                );
            });

            match dispatch_size.into_work_groups(dimensions) {
                Ok((x, y, z)) => {
                    field::weak_label(ui, format!("Resolves to dispatchWorkgroups({x}, {y}, {z})"))
                }
                Err(error) => field::error_label(ui, error.to_string()),
            };

            if dispatch_size != before {
                compute_pass.set_dispatch_size(dispatch_size);
            }
        },
    );
}

fn work_size_row(
    ui: &mut egui::Ui,
    label: &str,
    doc: impl FieldDoc,
    id_salt: &str,
    work_size: &mut WorkSize,
    dimensions: &Storage<Dimension>,
) {
    field::row_doc(ui, label, doc, |ui| {
        ui.horizontal(|ui| {
            let mut kind = WorkSizeKind::from_work_size(work_size);
            if inspector::value_combo(ui, (id_salt, "kind"), WORK_SIZE_KINDS, &mut kind) {
                *work_size = match kind {
                    WorkSizeKind::Fixed => WorkSize::Fixed(1),
                    WorkSizeKind::Dimension => WorkSize::Dimension(DimensionRef::default()),
                };
            }

            match work_size {
                WorkSize::Fixed(value) => {
                    egui::DragValue::new(value)
                        .speed(1)
                        .range(1..=u32::MAX)
                        .ui(ui);
                }
                WorkSize::Dimension(dimension_ref) => {
                    dimension_ref_edit(ui, (id_salt, "ref"), dimensions, dimension_ref);
                }
            }
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
    let mut policy = compute_pass.dispatch_policy();
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
