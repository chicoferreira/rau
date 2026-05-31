use std::ops::Range;

use egui::{CollapsingHeader, RichText};

use crate::{
    project::{
        BindGroupId, RenderPipelineId,
        resource::{
            bindgroup::BindGroup,
            model::Model,
            render_pipeline::{RenderDrawStrategy, RenderPipeline},
            shader::Shader,
        },
        storage::Storage,
    },
    ui::{
        components::{
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector,
            selector::AsWidgetText,
        },
        pane::StateSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrawKind {
    Direct,
    Model,
}

impl DrawKind {
    fn from_strategy(strategy: &RenderDrawStrategy) -> Self {
        match strategy {
            RenderDrawStrategy::Direct { .. } => Self::Direct,
            RenderDrawStrategy::Model { .. } => Self::Model,
        }
    }

    fn default_strategy(self) -> RenderDrawStrategy {
        match self {
            Self::Direct => RenderDrawStrategy::Direct {
                vertices: 0..3,
                instances: 0..1,
            },
            Self::Model => RenderDrawStrategy::Model {
                model_id: None,
                instances: 0..1,
                mesh_vertex_slot: 0,
                material_bind_group_slot: None,
            },
        }
    }
}

impl AsWidgetText for DrawKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            Self::Direct => "Direct",
            Self::Model => "Model",
        }
        .into()
    }
}

impl AsWidgetText for wgpu::PrimitiveTopology {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            wgpu::PrimitiveTopology::PointList => "Point List",
            wgpu::PrimitiveTopology::LineList => "Line List",
            wgpu::PrimitiveTopology::LineStrip => "Line Strip",
            wgpu::PrimitiveTopology::TriangleList => "Triangle List",
            wgpu::PrimitiveTopology::TriangleStrip => "Triangle Strip",
        }
        .into()
    }
}

impl AsWidgetText for Option<wgpu::IndexFormat> {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            None => "None",
            Some(wgpu::IndexFormat::Uint16) => "Uint16",
            Some(wgpu::IndexFormat::Uint32) => "Uint32",
        }
        .into()
    }
}

impl AsWidgetText for wgpu::FrontFace {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            wgpu::FrontFace::Ccw => "Counter-Clockwise",
            wgpu::FrontFace::Cw => "Clockwise",
        }
        .into()
    }
}

impl AsWidgetText for Option<wgpu::Face> {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            None => "None",
            Some(wgpu::Face::Front) => "Front",
            Some(wgpu::Face::Back) => "Back",
        }
        .into()
    }
}

impl AsWidgetText for wgpu::PolygonMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            wgpu::PolygonMode::Fill => "Fill",
            wgpu::PolygonMode::Line => "Line",
            wgpu::PolygonMode::Point => "Point",
        }
        .into()
    }
}

const DRAW_KINDS: [DrawKind; 2] = [DrawKind::Direct, DrawKind::Model];

const COLOR_FORMATS: [wgpu::TextureFormat; 4] = [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::Rgba32Float,
];

const DEPTH_FORMATS: [wgpu::TextureFormat; 3] = [
    wgpu::TextureFormat::Depth32Float,
    wgpu::TextureFormat::Depth24Plus,
    wgpu::TextureFormat::Depth24PlusStencil8,
];

const TOPOLOGIES: [wgpu::PrimitiveTopology; 5] = [
    wgpu::PrimitiveTopology::PointList,
    wgpu::PrimitiveTopology::LineList,
    wgpu::PrimitiveTopology::LineStrip,
    wgpu::PrimitiveTopology::TriangleList,
    wgpu::PrimitiveTopology::TriangleStrip,
];

const STRIP_INDEX_FORMATS: [Option<wgpu::IndexFormat>; 3] = [
    None,
    Some(wgpu::IndexFormat::Uint16),
    Some(wgpu::IndexFormat::Uint32),
];

const FRONT_FACES: [wgpu::FrontFace; 2] = [wgpu::FrontFace::Ccw, wgpu::FrontFace::Cw];

const CULL_MODES: [Option<wgpu::Face>; 3] = [None, Some(wgpu::Face::Front), Some(wgpu::Face::Back)];

const POLYGON_MODES: [wgpu::PolygonMode; 3] = [
    wgpu::PolygonMode::Fill,
    wgpu::PolygonMode::Line,
    wgpu::PolygonMode::Point,
];

impl StateSnapshot<'_> {
    pub fn render_pipeline_inspector_ui(
        &mut self,
        ui: &mut egui::Ui,
        render_pipeline_id: RenderPipelineId,
    ) {
        let shaders = &self.project.shaders;
        let bind_groups = &self.project.bind_groups;
        let models = &self.project.models;

        let Ok(render_pipeline) = self.project.render_pipelines.get_mut(render_pipeline_id) else {
            ui.label("Render Pipeline couldn't be found.");
            return;
        };

        shaders_ui(ui, render_pipeline_id, render_pipeline, shaders);
        ui.add_space(4.0);

        target_formats_ui(ui, render_pipeline_id, render_pipeline);
        ui.add_space(4.0);

        primitive_state_ui(ui, render_pipeline_id, render_pipeline);
        ui.add_space(4.0);

        static_bind_groups_ui(ui, render_pipeline_id, render_pipeline, bind_groups);
        ui.add_space(4.0);

        draw_strategy_ui(ui, render_pipeline_id, render_pipeline, models);
    }
}

fn shaders_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
    shaders: &Storage<Shader>,
) {
    CollapsingHeader::new("Shaders")
        .default_open(true)
        .show(ui, |ui| {
            let mut vertex_shader = render_pipeline.vertex_shader();
            let mut fragment_shader = render_pipeline.fragment_shader();

            inspector::field_grid(ui, (render_pipeline_id, "shaders"), |ui| {
                inspector::storage_opt_combo_row(
                    ui,
                    "Vertex Shader",
                    "render_pipeline_vertex_shader",
                    shaders,
                    &mut vertex_shader,
                );
                inspector::storage_opt_combo_row(
                    ui,
                    "Fragment Shader",
                    "render_pipeline_fragment_shader",
                    shaders,
                    &mut fragment_shader,
                );
            });

            render_pipeline.set_vertex_shader(vertex_shader);
            render_pipeline.set_fragment_shader(fragment_shader);
        });
}

fn target_formats_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
) {
    CollapsingHeader::new("Target Formats")
        .default_open(true)
        .show(ui, |ui| {
            let mut color_format = render_pipeline.color_format();
            let mut depth_format = render_pipeline.depth_format();

            inspector::field_grid(ui, (render_pipeline_id, "target_formats"), |ui| {
                inspector::combo_row(
                    ui,
                    "Color Format",
                    "render_pipeline_color_format",
                    COLOR_FORMATS,
                    &mut color_format,
                );

                let mut depth_enabled = depth_format.is_some();
                if inspector::checkbox_row(ui, "Depth", &mut depth_enabled) {
                    depth_format = depth_enabled.then_some(wgpu::TextureFormat::Depth32Float);
                }

                if let Some(format) = &mut depth_format {
                    inspector::combo_row(
                        ui,
                        "Depth Format",
                        "render_pipeline_depth_format",
                        DEPTH_FORMATS,
                        format,
                    );
                }
            });

            render_pipeline.set_color_format(color_format);
            render_pipeline.set_depth_format(depth_format);
        });
}

fn primitive_state_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
) {
    CollapsingHeader::new("Primitive State")
        .default_open(true)
        .show(ui, |ui| {
            let mut primitive_state = render_pipeline.primitive_state();

            inspector::field_grid(ui, (render_pipeline_id, "primitive_state"), |ui| {
                inspector::combo_row(
                    ui,
                    "Topology",
                    "render_pipeline_topology",
                    TOPOLOGIES,
                    &mut primitive_state.topology,
                );
                inspector::combo_row(
                    ui,
                    "Strip Index Format",
                    "render_pipeline_strip_index_format",
                    STRIP_INDEX_FORMATS,
                    &mut primitive_state.strip_index_format,
                );
                inspector::combo_row(
                    ui,
                    "Front Face",
                    "render_pipeline_front_face",
                    FRONT_FACES,
                    &mut primitive_state.front_face,
                );
                inspector::combo_row(
                    ui,
                    "Cull Mode",
                    "render_pipeline_cull_mode",
                    CULL_MODES,
                    &mut primitive_state.cull_mode,
                );
                inspector::combo_row(
                    ui,
                    "Polygon Mode",
                    "render_pipeline_polygon_mode",
                    POLYGON_MODES,
                    &mut primitive_state.polygon_mode,
                );
                inspector::checkbox_row(
                    ui,
                    "Unclipped Depth",
                    &mut primitive_state.unclipped_depth,
                );
                inspector::checkbox_row(ui, "Conservative", &mut primitive_state.conservative);
            });

            render_pipeline.set_primitive_state(primitive_state);
        });
}

fn static_bind_groups_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
    bind_groups: &Storage<BindGroup>,
) {
    let before = render_pipeline.static_bind_groups().to_vec();
    let mut entries = before.clone();

    CollapsingHeader::new(format!("Static Bind Groups ({})", entries.len()))
        .default_open(true)
        .show(ui, |ui| {
            if entries.is_empty() {
                ui.label("No static bind groups.");
            }

            let id_source = (render_pipeline_id, "static_bind_groups");
            let mut edits = draggable_list(
                ui,
                id_source,
                &entries,
                |ui, (slot, bind_group_id), index, handle, edits| {
                    static_bind_group_row_ui(
                        ui,
                        handle,
                        index,
                        *slot,
                        *bind_group_id,
                        bind_groups,
                        edits,
                    );
                },
            );

            if ui.button("Add Bind Group").clicked() {
                let first_slot = first_available_slot(entries.iter().map(|(slot, _)| *slot));
                edits.push_add_edit((first_slot, None));
            }

            if !entries.is_empty() {
                ui.add_space(6.0);
                ui.add(hint(|ui| {
                    ui.label("Right-click a");
                    ui.label(RichText::new("Slot").strong());
                    ui.label("to remove it, or drag to reorder.");
                }));
            }

            edits.apply(&mut entries);

            if entries != before {
                render_pipeline.set_static_bind_groups(entries);
            }
        });
}

fn static_bind_group_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    slot: u32,
    bind_group_id: Option<BindGroupId>,
    bind_groups: &Storage<BindGroup>,
    edits: &mut ListEdits<(u32, Option<BindGroupId>)>,
) {
    handle.ui(ui, |ui| {
        ui.add(
            egui::Label::new(format!("Slot {slot}"))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Remove Bind Group").clicked() {
                edits.push_remove_edit(index);
                ui.close();
            }
        });
    });

    ui.indent(("static_bind_group", index), |ui| {
        let mut edited_slot = slot;
        let mut edited_bind_group_id = bind_group_id;

        inspector::field_grid(ui, ("static_bind_group_grid", index), |ui| {
            inspector::u32_drag_row(ui, "Slot", &mut edited_slot, 0..=u32::MAX);
            inspector::storage_opt_combo_row(
                ui,
                "Bind Group",
                "render_pipeline_static_bind_group",
                bind_groups,
                &mut edited_bind_group_id,
            );
        });

        if (edited_slot, edited_bind_group_id) != (slot, bind_group_id) {
            edits.push_set_edit(index, (edited_slot, edited_bind_group_id));
        }
    });
}

fn draw_strategy_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
    models: &Storage<Model>,
) {
    let before = render_pipeline.draw_strategy().clone();
    let mut edited = before.clone();
    let static_bind_group_slots = render_pipeline
        .static_bind_groups()
        .iter()
        .map(|(slot, _)| *slot)
        .collect::<Vec<_>>();

    CollapsingHeader::new("Draw")
        .default_open(true)
        .show(ui, |ui| {
            inspector::field_grid(ui, (render_pipeline_id, "draw_strategy"), |ui| {
                let mut draw_kind = DrawKind::from_strategy(&edited);
                if inspector::combo_row(
                    ui,
                    "Draw Kind",
                    "render_pipeline_draw_kind",
                    DRAW_KINDS,
                    &mut draw_kind,
                ) {
                    edited = draw_kind.default_strategy();
                }

                draw_strategy_fields_ui(ui, models, &static_bind_group_slots, &mut edited);
            });
        });

    if edited != before {
        render_pipeline.set_draw_strategy(edited);
    }
}

fn draw_strategy_fields_ui(
    ui: &mut egui::Ui,
    models: &Storage<Model>,
    static_bind_group_slots: &[u32],
    draw_strategy: &mut RenderDrawStrategy,
) {
    match draw_strategy {
        RenderDrawStrategy::Direct {
            vertices,
            instances,
        } => {
            inspector::row(ui, "Vertices", |ui| range_u32_edit(ui, vertices));
            inspector::row(ui, "Instances", |ui| range_u32_edit(ui, instances));
        }
        RenderDrawStrategy::Model {
            model_id,
            instances,
            mesh_vertex_slot,
            material_bind_group_slot,
        } => {
            inspector::storage_opt_combo_row(
                ui,
                "Model",
                "render_pipeline_model",
                models,
                model_id,
            );
            inspector::row(ui, "Instances", |ui| range_u32_edit(ui, instances));
            inspector::u32_drag_row(ui, "Mesh Vertex Slot", mesh_vertex_slot, 0..=u32::MAX);

            let mut material_enabled = material_bind_group_slot.is_some();
            if inspector::checkbox_row(ui, "Material Bind Group", &mut material_enabled) {
                *material_bind_group_slot = material_enabled
                    .then(|| first_available_slot(static_bind_group_slots.iter().copied()));
            }

            if let Some(slot) = material_bind_group_slot {
                inspector::u32_drag_row(ui, "Material Slot", slot, 0..=u32::MAX);
            }
        }
    }
}

fn range_u32_edit(ui: &mut egui::Ui, range: &mut Range<u32>) {
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut range.start).range(0..=range.end));
        ui.label("..");
        ui.add(egui::DragValue::new(&mut range.end).range(range.start..=u32::MAX));
    });
}

fn first_available_slot(used: impl IntoIterator<Item = u32>) -> u32 {
    let mut used = used.into_iter().collect::<Vec<_>>();
    used.sort_unstable();

    let mut candidate = 0;
    for slot in used {
        if slot == candidate {
            candidate += 1;
        } else if slot > candidate {
            break;
        }
    }

    candidate
}
