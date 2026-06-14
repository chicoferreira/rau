use std::ops::Range;

use egui::RichText;

use crate::{
    project::{
        RenderPipelineId,
        resource::{
            bindgroup::BindGroup,
            model::Model,
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            shader::Shader,
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            draggable_list::{ListEdits, draggable_list},
            hint::hint,
            inspector::{self, AsWidgetText},
        },
        pane::StateSnapshot,
    },
    utils::{shader_preview::ShaderGenCtx, texture_format::TextureFormat},
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

        target_formats_ui(ui, render_pipeline_id, render_pipeline);

        primitive_state_ui(ui, render_pipeline_id, render_pipeline);

        bind_groups_ui(ui, render_pipeline_id, render_pipeline, bind_groups);

        draw_strategy_ui(ui, render_pipeline_id, render_pipeline, models);

        if let Ok(pipeline) = self.project.render_pipelines.get(render_pipeline_id) {
            let ctx = ShaderGenCtx::from_project(self.project);
            shader_code_section(ui, pipeline, &ctx);
        }
    }
}

fn shaders_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
    shaders: &Storage<Shader>,
) {
    let mut vertex_shader = render_pipeline.vertex_shader();
    let mut fragment_shader = render_pipeline.fragment_shader();

    inspector::section(ui, "Shaders", |ui| {
        inspector::field_grid(ui, (render_pipeline_id, "shaders"), |ui| {
            inspector::storage_combo_row(
                ui,
                "Vertex Shader",
                "render_pipeline_vertex_shader",
                shaders,
                &mut vertex_shader,
            );
            inspector::storage_combo_row(
                ui,
                "Fragment Shader",
                "render_pipeline_fragment_shader",
                shaders,
                &mut fragment_shader,
            );
        });
    });

    render_pipeline.set_vertex_shader(vertex_shader);
    render_pipeline.set_fragment_shader(fragment_shader);
}

fn target_formats_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
) {
    let mut color_format = render_pipeline.color_format();
    let mut depth_format = render_pipeline.depth_format();

    inspector::section(ui, "Target Formats", |ui| {
        inspector::field_grid(ui, (render_pipeline_id, "target_formats"), |ui| {
            inspector::combo_row(
                ui,
                "Color Format",
                "render_pipeline_color_format",
                TextureFormat::COLOR,
                &mut color_format,
            );

            let mut depth_enabled = depth_format.is_some();
            if inspector::checkbox_row(ui, "Depth", &mut depth_enabled) {
                depth_format = depth_enabled.then_some(TextureFormat::Depth32Float);
            }

            if let Some(format) = &mut depth_format {
                inspector::combo_row(
                    ui,
                    "Depth Format",
                    "render_pipeline_depth_format",
                    TextureFormat::DEPTH,
                    format,
                );
            }
        });
    });

    render_pipeline.set_color_format(color_format);
    render_pipeline.set_depth_format(depth_format);
}

fn primitive_state_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
) {
    let mut primitive_state = render_pipeline.primitive_state();

    inspector::section(ui, "Primitive State", |ui| {
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
            inspector::checkbox_row(ui, "Unclipped Depth", &mut primitive_state.unclipped_depth);
            inspector::checkbox_row(ui, "Conservative", &mut primitive_state.conservative);
        });
    });

    render_pipeline.set_primitive_state(primitive_state);
}

fn bind_groups_ui(
    ui: &mut egui::Ui,
    render_pipeline_id: RenderPipelineId,
    render_pipeline: &mut RenderPipeline,
    bind_groups: &Storage<BindGroup>,
) {
    let before = render_pipeline.bind_groups().to_vec();
    let mut entries = before.clone();

    inspector::section(ui, &format!("Bind Groups ({})", entries.len()), |ui| {
        if entries.is_empty() {
            ui.label("No bind groups.");
        }

        let id_source = (render_pipeline_id, "bind_groups");
        let mut edits = draggable_list(
            ui,
            id_source,
            &entries,
            |ui, target, index, handle, edits| {
                bind_group_row_ui(ui, handle, index, target, bind_groups, edits);
            },
        );

        ui.add_space(3.0);
        if ui.button("Add Bind Group").clicked() {
            edits.push_add_edit(BindGroupTarget::default());
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
            render_pipeline.set_bind_groups(entries);
        }
    });
}

fn bind_group_row_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    index: usize,
    target: &BindGroupTarget,
    bind_groups: &Storage<BindGroup>,
    edits: &mut ListEdits<BindGroupTarget>,
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

    ui.indent(("bind_group", index), |ui| {
        let mut edited = target.clone();

        bind_group_target_combo(ui, index, bind_groups, &mut edited);

        if edited != *target {
            edits.push_set_edit(index, edited);
        }
    });
}

fn bind_group_target_combo(
    ui: &mut egui::Ui,
    index: usize,
    bind_groups: &Storage<BindGroup>,
    target: &mut BindGroupTarget,
) {
    let selected_text: egui::WidgetText = match target {
        BindGroupTarget::Empty => "Empty".into(),
        BindGroupTarget::ModelMaterial => "Model Material".into(),
        BindGroupTarget::Static(id) => match bind_groups.get_label(*id) {
            Ok(label) => label.into(),
            Err(_) => format!("Unknown {id:?}").into(),
        },
    };

    egui::ComboBox::from_id_salt(("bind_group_target", index))
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            ui.selectable_value(target, BindGroupTarget::Empty, "Empty");
            ui.selectable_value(target, BindGroupTarget::ModelMaterial, "Model Material")
                .on_hover_text(
                    "Binds each mesh's active material bind group to this slot while drawing. \
                     Only has an effect with a Model draw strategy.",
                );
            ui.separator();
            for (id, bind_group) in bind_groups.list() {
                let selected = *target == BindGroupTarget::Static(id);
                if ui.selectable_label(selected, bind_group.label()).clicked() {
                    *target = BindGroupTarget::Static(id);
                }
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

    inspector::section(ui, "Draw", |ui| {
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

            draw_strategy_fields_ui(ui, models, &mut edited);
        });
    });

    if edited != before {
        render_pipeline.set_draw_strategy(edited);
    }
}

fn draw_strategy_fields_ui(
    ui: &mut egui::Ui,
    models: &Storage<Model>,
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
        } => {
            inspector::storage_combo_row(
                ui,
                "Model",
                "render_pipeline_model",
                models,
                model_id,
            );
            inspector::row(ui, "Instances", |ui| range_u32_edit(ui, instances));
            inspector::u32_drag_row(ui, "Mesh Vertex Slot", mesh_vertex_slot, 0..=u32::MAX);
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
