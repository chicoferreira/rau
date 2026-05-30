use egui::{CollapsingHeader, RichText, Widget};

use crate::{
    project::{
        ProjectResource, RenderPassId, RenderPipelineId, TextureViewId,
        resource::{
            render_pass::{LoadOperation, RenderPassTarget},
            render_pipeline::RenderPipeline,
            texture_view::TextureView,
        },
        storage::Storage,
    },
    ui::{
        components::{
            color_edit::color_edit_rgba,
            hint::hint,
            inspector,
            selector::{AsWidgetText, ComboBoxExt},
        },
        pane::StateSnapshot,
    },
    workspace::StateEvent,
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
        let event_queue = &mut *self.event_queue;

        let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) else {
            ui.label("Render Pass couldn't be found.");
            return;
        };

        CollapsingHeader::new("Color Target")
            .default_open(true)
            .show(ui, |ui| {
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
            });

        ui.add_space(4.0);

        CollapsingHeader::new("Depth Target")
            .default_open(true)
            .show(ui, |ui| {
                let mut enabled = render_pass.depth_target().is_some();
                if inspector::checkbox_row(ui, "Enabled", &mut enabled) {
                    if enabled {
                        render_pass.set_depth_target(Some(RenderPassTarget::default()));
                    } else {
                        render_pass.set_depth_target(None);
                    }
                }

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
            });

        ui.add_space(4.0);

        CollapsingHeader::new(format!("Pipelines ({})", render_pass.pipelines().len()))
            .default_open(true)
            .show(ui, |ui| {
                render_pass_pipeline_list_ui(
                    ui,
                    render_pass_id,
                    render_pass,
                    render_pipelines,
                    event_queue,
                );
            });
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
        inspector::storage_opt_combo_row(
            ui,
            "Texture View",
            (id_salt, "texture_view"),
            texture_views,
            texture_view_id,
        );

        inspector::row(ui, "Load Operation", |ui| {
            let kind_before = load_op_kind(load_op);
            let mut kind = kind_before;

            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt((id_salt, "load_operation"))
                    .selected_text(kind.as_widget_text())
                    .show_ui_list(ui, LOAD_OP_KINDS, &mut kind);

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
        });
    });

    (*texture_view_id, *load_op) != before
}

fn render_pass_pipeline_list_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    render_pass: &mut crate::project::resource::render_pass::RenderPass,
    render_pipelines: &Storage<RenderPipeline>,
    event_queue: &mut crate::utils::event_queue::EventQueue<StateEvent>,
) {
    let before = render_pass.pipelines().to_vec();
    let mut pipelines = before.clone();
    let mut edits = Vec::new();

    if pipelines.is_empty() {
        ui.label("No pipelines in render pass.");
    }

    let response = inspector::field_grid(ui, (render_pass_id, "render_pass_pipeline_grid"), |ui| {
        egui_dnd::dnd(ui, (render_pass_id, "render_pass_pipelines")).show_custom(|ui, iter| {
            for (index, pipeline_id) in pipelines.iter().copied().enumerate() {
                let item_id = egui::Id::new((render_pass_id, pipeline_id, index));
                ui.push_id((pipeline_id, index), |ui| {
                    iter.next(ui, item_id, index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            render_pass_pipeline_row_ui(
                                ui,
                                handle,
                                index,
                                pipeline_id,
                                render_pipelines,
                                event_queue,
                                &mut edits,
                            );
                        })
                    });
                });
            }
        })
    })
    .inner;

    if let Some(update) = response.final_update() {
        edits.push(RenderPassPipelineEdit::Reorder(update));
    }

    ui.add_space(6.0);
    render_pass_add_pipeline_ui(ui, render_pipelines, &mut edits);

    apply_render_pass_pipeline_edits(&mut pipelines, edits);
    let has_pipelines = !pipelines.is_empty();

    if pipelines != before {
        render_pass.set_pipelines(pipelines);
    }

    if has_pipelines {
        ui.add_space(6.0);
        ui.add(hint(|ui| {
            ui.label("Right-click a");
            ui.label(RichText::new("Pipeline").strong());
            ui.label("to inspect or remove it, or drag to reorder.");
        }));
    }
}

fn render_pass_pipeline_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    pipeline_id: RenderPipelineId,
    render_pipelines: &Storage<RenderPipeline>,
    event_queue: &mut crate::utils::event_queue::EventQueue<StateEvent>,
    edits: &mut Vec<RenderPassPipelineEdit>,
) {
    handle.ui(ui, |ui| {
        let mut selected = pipeline_id;

        let response = inspector::row(ui, format!("{}.", index + 1), |ui| {
            egui::ComboBox::from_id_salt(("render_pass_pipeline_select", index))
                .selected_text(render_pipeline_label(render_pipelines, selected))
                .show_ui(ui, |ui| {
                    for (id, pipeline) in render_pipelines.list_sorted() {
                        ui.selectable_value(&mut selected, id, pipeline.label());
                    }
                })
                .response
        });

        response.context_menu(|ui| {
            if ui.button("Inspect Pipeline").clicked() {
                event_queue.add(StateEvent::InspectResource(pipeline_id.into()));
                ui.close();
            }

            if ui.button("Remove Pipeline").clicked() {
                edits.push(RenderPassPipelineEdit::Remove(index));
                ui.close();
            }
        });

        if selected != pipeline_id {
            edits.push(RenderPassPipelineEdit::Update(index, selected));
        }
    });
}

fn apply_render_pass_pipeline_edits(
    pipelines: &mut Vec<RenderPipelineId>,
    edits: Vec<RenderPassPipelineEdit>,
) {
    for edit in edits {
        match edit {
            RenderPassPipelineEdit::Add(id) => pipelines.push(id),
            RenderPassPipelineEdit::Update(index, id) => {
                if let Some(pipeline) = pipelines.get_mut(index) {
                    *pipeline = id;
                }
            }
            RenderPassPipelineEdit::Remove(index) => {
                if index < pipelines.len() {
                    pipelines.remove(index);
                }
            }
            RenderPassPipelineEdit::Reorder(update) => {
                egui_dnd::utils::shift_vec(update.from, update.to, pipelines);
            }
        }
    }
}

fn render_pass_add_pipeline_ui(
    ui: &mut egui::Ui,
    render_pipelines: &Storage<RenderPipeline>,
    edits: &mut Vec<RenderPassPipelineEdit>,
) {
    ui.menu_button("Add Pipeline", |ui| {
        let mut has_pipelines = false;
        for (id, pipeline) in render_pipelines.list_sorted() {
            has_pipelines = true;
            if ui.button(pipeline.label()).clicked() {
                edits.push(RenderPassPipelineEdit::Add(id));
                ui.close();
            }
        }

        if !has_pipelines {
            ui.label("No render pipelines.");
        }
    });
}

fn render_pipeline_label(
    render_pipelines: &Storage<RenderPipeline>,
    pipeline_id: RenderPipelineId,
) -> String {
    render_pipelines
        .get_label(pipeline_id)
        .map(str::to_owned)
        .unwrap_or_else(|_| format!("Unknown {pipeline_id:?}"))
}

enum RenderPassPipelineEdit {
    Add(RenderPipelineId),
    Update(usize, RenderPipelineId),
    Remove(usize),
    Reorder(egui_dnd::DragUpdate),
}
