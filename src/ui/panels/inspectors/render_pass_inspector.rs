use egui::Widget;

use crate::{
    project::{
        RenderPassId, RenderPipelineId, TextureViewId,
        resource::{
            render_pass::{LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::RenderPipeline,
            texture_view::TextureView,
        },
        storage::Storage,
    },
    ui::{
        components::{
            color_edit::color_edit_rgba,
            draggable_list::{ListEdits, draggable_list},
            field_docs::field_doc,
            inspector::{self, AsWidgetText},
        },
        pane::StateSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadOpKind {
    Clear,
    Load,
}

impl AsWidgetText for LoadOpKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            LoadOpKind::Clear => "Clear",
            LoadOpKind::Load => "Load",
        }
        .into()
    }
}

const LOAD_OP_KINDS: [LoadOpKind; 2] = [LoadOpKind::Clear, LoadOpKind::Load];

fn load_op_kind<T>(op: &LoadOperation<T>) -> LoadOpKind {
    match op {
        LoadOperation::Clear(_) => LoadOpKind::Clear,
        LoadOperation::Load => LoadOpKind::Load,
    }
}

impl StateSnapshot<'_> {
    pub fn render_pass_inspector_ui(&mut self, ui: &mut egui::Ui, render_pass_id: RenderPassId) {
        let texture_views = &self.project.texture_views;
        let render_pipelines = &self.project.render_pipelines;

        let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) else {
            ui.label("Render Pass couldn't be found.");
            return;
        };

        inspector::section_doc(
            ui,
            "Color Target",
            field_doc!(
                "The color attachment this pass draws into: a Texture View and how its existing \
                contents are treated at the start of the pass.\n\n\
                [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpurenderpasscolorattachment)"
            ),
            |ui| {
                let target = render_pass.target();
                let mut texture_view_id = target.texture_view_id();
                let mut load_op = target.load_operation();

                if render_pass_target_ui(
                    ui,
                    "color_target",
                    texture_views,
                    &mut texture_view_id,
                    &mut load_op,
                    |ui, color| {
                        color_edit_rgba(ui, &mut color.0);
                    },
                ) {
                    render_pass.set_target(RenderPassTarget::new(texture_view_id, load_op));
                }
            },
        );

        inspector::section_doc(
            ui,
            "Depth Target",
            field_doc!(
                "The optional depth attachment used for depth testing while this pass draws.\n\n\
                [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpurenderpassdepthstencilattachment)"
            ),
            |ui| {
                let mut enabled = render_pass.depth_target().is_some();
                ui.horizontal(|ui| {
                    if inspector::checkbox_row_doc(
                        ui,
                        "Enabled",
                        field_doc!("Whether this pass writes to and tests against a depth buffer."),
                        &mut enabled,
                    ) {
                        if enabled {
                            render_pass.set_depth_target(Some(RenderPassTarget::default()));
                        } else {
                            render_pass.set_depth_target(None);
                        }
                    }
                });

                let depth_target = render_pass
                    .depth_target()
                    .map(|target| (target.texture_view_id(), target.load_operation()));

                if let Some((mut texture_view_id, mut load_op)) = depth_target
                    && render_pass_target_ui(
                        ui,
                        "depth_target",
                        texture_views,
                        &mut texture_view_id,
                        &mut load_op,
                        |ui, value| {
                            egui::DragValue::new(value)
                                .speed(0.001)
                                .range(0.0..=1.0)
                                .max_decimals(4)
                                .ui(ui);
                        },
                    )
                {
                    render_pass
                        .set_depth_target(Some(RenderPassTarget::new(texture_view_id, load_op)));
                }
            },
        );

        inspector::section_doc_wide(
            ui,
            &format!("Pipelines ({})", render_pass.pipelines().len()),
            field_doc!(
                r"The Render Pipelines run by this pass, in order. Each step binds its pipeline and issues its draw.

Roughly, the pass executes:

```rs
begin(color_target, depth_target)
for pipeline in pipelines:
  run(pipeline) // see the pipeline's 'Draw' section
```

Drag to reorder, right-click to remove.

[wgpu RenderPass](https://docs.rs/wgpu/latest/wgpu/struct.RenderPass.html)"
            ),
            |ui| {
                render_pass_pipeline_list_ui(ui, render_pass_id, render_pass, render_pipelines);
            },
        );
    }
}

fn render_pass_target_ui<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    texture_views: &Storage<TextureView>,
    texture_view_id: &mut Option<TextureViewId>,
    load_op: &mut LoadOperation<T>,
    clear_value_ui: impl FnOnce(&mut egui::Ui, &mut T),
) -> bool
where
    LoadOperation<T>: Default,
{
    let before = (*texture_view_id, *load_op);

    inspector::field_grid(ui, (id_salt, "target_grid"), |ui| {
        inspector::row_doc(
            ui,
            "Texture View",
            field_doc!("The Texture View this target renders into."),
            |ui| {
                inspector::storage_combo(
                    ui,
                    (id_salt, "texture_view"),
                    texture_views,
                    texture_view_id,
                )
            },
        );

        inspector::row_doc(
            ui,
            "Load Operation",
            field_doc!(
                "What happens to the target's existing contents at the **start** of the \
                pass:\n\n\
                - **Clear**: replace with the clear value (set alongside).\n\
                - **Load**: keep the previous contents and draw over them.\n\n\
                [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpuloadop)"
            ),
            |ui| {
                let kind_before = load_op_kind(load_op);
                let mut kind = kind_before;

                ui.horizontal(|ui| {
                    inspector::value_combo(
                        ui,
                        (id_salt, "load_operation"),
                        LOAD_OP_KINDS,
                        &mut kind,
                    );

                    if kind != kind_before {
                        *load_op = match kind {
                            LoadOpKind::Clear => LoadOperation::default(),
                            LoadOpKind::Load => LoadOperation::Load,
                        };
                    }

                    if let LoadOperation::Clear(value) = load_op {
                        clear_value_ui(ui, value);
                    }
                });
            },
        );
    });

    (*texture_view_id, *load_op) != before
}

fn render_pass_pipeline_list_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    render_pass: &mut RenderPass,
    render_pipelines: &Storage<RenderPipeline>,
) {
    let before = render_pass.pipelines().to_vec();
    let mut pipelines = before.clone();

    if pipelines.is_empty() {
        ui.label("No pipelines in render pass.");
    }

    let mut edits = draggable_list(
        ui,
        (render_pass_id, "render_pass_pipeline_grid"),
        &pipelines,
        |ui, pipeline_id, index, handle, edits| {
            render_pass_pipeline_row_ui(ui, handle, index, *pipeline_id, render_pipelines, edits);
        },
    );

    ui.add_space(3.0);

    inspector::add_from_storage_menu(
        ui,
        "Add Pipeline",
        render_pipelines,
        "No render pipelines.",
        |id| edits.push_add_edit(id),
    );

    edits.apply(&mut pipelines);

    if pipelines != before {
        render_pass.set_pipelines(pipelines);
    }
}

fn render_pass_pipeline_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    pipeline_id: RenderPipelineId,
    render_pipelines: &Storage<RenderPipeline>,
    edits: &mut ListEdits<RenderPipelineId>,
) {
    handle.ui(ui, |ui| {
        ui.add(egui::Label::new(format!("Step {}", index + 1)).sense(egui::Sense::click()))
            .context_menu(|ui| {
                if ui.button("Remove Pipeline").clicked() {
                    edits.push_remove_edit(index);
                    ui.close();
                }
            });
    });

    let mut selected = pipeline_id;

    ui.indent(("render_pass_pipeline_select", index), |ui| {
        inspector::storage_id_combo(
            ui,
            ("render_pass_pipeline_select", index),
            render_pipelines,
            &mut selected,
        );
    });

    if selected != pipeline_id {
        edits.push_set_edit(index, selected);
    }
}
