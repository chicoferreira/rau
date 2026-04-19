use egui::{CollapsingHeader, Grid, RichText, Widget};
use egui_dnd::DragDropItem;

use crate::{
    project::{
        RenderPassId, TextureViewId,
        render_pass::{LoadOperation, RenderPassTarget},
        storage::Storage,
        texture_view::TextureView,
    },
    state::StateEvent,
    ui::{
        components::{
            color_edit::color_edit_rgba,
            hint::hint,
            selector::{AsWidgetText, ComboBoxExt},
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
        let Ok(render_pass) = self.project.render_passes.get_mut(render_pass_id) else {
            ui.label("Render Pass couldn't be found.");
            return;
        };

        CollapsingHeader::new("Color Target")
            .default_open(true)
            .show(ui, |ui| {
                let mut texture_view_id = render_pass.target.texture_view_id;
                let mut load_op = render_pass.target.load_operation;

                let changed = render_pass_target_ui(
                    ui,
                    "color_target",
                    &self.project.texture_views,
                    &mut texture_view_id,
                    &mut load_op,
                    |ui, color| {
                        color_edit_rgba(ui, &mut color.0);
                    },
                );

                if changed {
                    render_pass.set_target(RenderPassTarget {
                        texture_view_id,
                        load_operation: load_op,
                    });
                }
            });

        ui.add_space(4.0);
        CollapsingHeader::new("Depth Target")
            .default_open(true)
            .show(ui, |ui| {
                let mut enabled = render_pass.depth_target.is_some();
                if ui.checkbox(&mut enabled, "Enabled").changed() {
                    if enabled {
                        render_pass.set_depth_target(Some(RenderPassTarget::default()));
                    } else {
                        render_pass.set_depth_target(None);
                    }
                }

                let depth_snapshot = render_pass
                    .depth_target
                    .as_ref()
                    .map(|dt| (dt.texture_view_id, dt.load_operation));

                if let Some((mut texture_view_id, mut load_op)) = depth_snapshot {
                    let changed = render_pass_target_ui(
                        ui,
                        "depth_target",
                        &self.project.texture_views,
                        &mut texture_view_id,
                        &mut load_op,
                        |ui, value| {
                            egui::DragValue::new(value)
                                .speed(0.001)
                                .range(0.0..=1.0)
                                .max_decimals(4)
                                .ui(ui);
                        },
                    );

                    if changed {
                        render_pass.set_depth_target(Some(RenderPassTarget {
                            texture_view_id,
                            load_operation: load_op,
                        }));
                    }
                }
            });

        ui.add_space(4.0);

        let pipeline_count = render_pass.pipelines.len();
        let shaders = &self.project.shaders;
        let bind_groups = &self.project.bind_groups;
        let models = &self.project.models;
        let pipelines = &mut render_pass.pipelines;
        let pending_events = &mut *self.pending_events;
        let rename_state = &mut *self.rename_state;

        CollapsingHeader::new(format!("Pipelines ({})", pipeline_count))
            .default_open(true)
            .show(ui, |ui| {
                let response =
                    egui_dnd::dnd(ui, (render_pass_id, "pipelines")).show_custom(|ui, iter| {
                        for (index, pipeline) in pipelines.iter_mut().enumerate() {
                            if index != 0 {
                                ui.add_space(5.0);
                            }
                            let item_id = pipeline.id();
                            ui.push_id(pipeline.id, |ui| {
                                iter.next(ui, item_id, index, true, |ui, item_handle| {
                                    item_handle.ui(ui, |ui, handle, _state| {
                                        super::pipeline::pipeline_entry_ui(
                                            ui,
                                            handle,
                                            render_pass_id,
                                            index,
                                            pipeline,
                                            shaders,
                                            bind_groups,
                                            models,
                                            pending_events,
                                            rename_state,
                                        );
                                    })
                                });
                            });
                        }
                    });

                if let Some(update) = response.final_update() {
                    pending_events.push(StateEvent::ReorderRenderPipeline(render_pass_id, update));
                }

                ui.add_space(6.0);

                if ui.button("Add Pipeline").clicked() {
                    pending_events.push(StateEvent::CreateRenderPipeline(render_pass_id));
                }

                if pipeline_count > 0 {
                    ui.add_space(6.0);
                    ui.add(hint(|ui| {
                        ui.label("Right-click a pipeline's");
                        ui.label(RichText::new("title").strong());
                        ui.label("to delete it, or drag it to reorder.");
                    }));
                }
            });
    }
}

fn render_pass_target_ui<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id_salt: &str,
    texture_views: &Storage<TextureView>,
    texture_view_id: &mut Option<TextureViewId>,
    load_op: &mut LoadOperation<T>,
    clear_value_ui: impl FnOnce(&mut egui::Ui, &mut T),
) -> bool
where
    LoadOperation<T>: Default,
{
    let tv_before = *texture_view_id;
    let load_op_before = *load_op;

    Grid::new((id_salt, "target_grid"))
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Texture View");
            let mut selected_texture_view_id = *texture_view_id;
            egui::ComboBox::from_id_salt((id_salt, "tv"))
                .selected_text_storage_opt(texture_views, selected_texture_view_id)
                .show_ui_storage_opt(ui, texture_views, &mut selected_texture_view_id);
            *texture_view_id = selected_texture_view_id;
            ui.end_row();

            ui.label("Load Operation");
            let kind_before = load_op_kind(load_op);
            let mut kind = kind_before;

            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt((id_salt, "load_op_kind"))
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
            ui.end_row();
        });

    *texture_view_id != tv_before || *load_op != load_op_before
}
