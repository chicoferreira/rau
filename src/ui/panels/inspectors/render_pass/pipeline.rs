use egui::{CollapsingHeader, Grid, Label, Sense};

use crate::{
    project::{
        ProjectResource, RenderPassId,
        resource::{
            bindgroup::BindGroup, model::Model, render_pass::RenderPipeline, shader::Shader,
        },
        storage::Storage,
    },
    state::StateEvent,
    ui::{
        components::{
            renameable_label::renameable_label,
            selector::{AsWidgetText, ComboBoxExt},
        },
        rename::{RenameState, RenameTarget},
    },
};

impl AsWidgetText for wgpu::PrimitiveTopology {
    fn as_widget_text(&self) -> egui::WidgetText {
        let s = match self {
            wgpu::PrimitiveTopology::PointList => "Point List",
            wgpu::PrimitiveTopology::LineList => "Line List",
            wgpu::PrimitiveTopology::LineStrip => "Line Strip",
            wgpu::PrimitiveTopology::TriangleList => "Triangle List",
            wgpu::PrimitiveTopology::TriangleStrip => "Triangle Strip",
        };
        s.into()
    }
}

impl AsWidgetText for wgpu::FrontFace {
    fn as_widget_text(&self) -> egui::WidgetText {
        let s = match self {
            wgpu::FrontFace::Ccw => "Counter-Clockwise",
            wgpu::FrontFace::Cw => "Clockwise",
        };
        s.into()
    }
}

impl AsWidgetText for Option<wgpu::Face> {
    fn as_widget_text(&self) -> egui::WidgetText {
        let s = match self {
            None => "None",
            Some(wgpu::Face::Front) => "Front",
            Some(wgpu::Face::Back) => "Back",
        };
        s.into()
    }
}

impl AsWidgetText for wgpu::PolygonMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        let s = match self {
            wgpu::PolygonMode::Fill => "Fill",
            wgpu::PolygonMode::Line => "Line",
            wgpu::PolygonMode::Point => "Point",
        };
        s.into()
    }
}

const TOPOLOGIES: [wgpu::PrimitiveTopology; 5] = [
    wgpu::PrimitiveTopology::PointList,
    wgpu::PrimitiveTopology::LineList,
    wgpu::PrimitiveTopology::LineStrip,
    wgpu::PrimitiveTopology::TriangleList,
    wgpu::PrimitiveTopology::TriangleStrip,
];

const FRONT_FACES: [wgpu::FrontFace; 2] = [wgpu::FrontFace::Ccw, wgpu::FrontFace::Cw];

const CULL_MODES: [Option<wgpu::Face>; 3] = [None, Some(wgpu::Face::Front), Some(wgpu::Face::Back)];

const POLYGON_MODES: [wgpu::PolygonMode; 3] = [
    wgpu::PolygonMode::Fill,
    wgpu::PolygonMode::Line,
    wgpu::PolygonMode::Point,
];

fn combo_list<T: Copy + PartialEq + AsWidgetText, I: IntoIterator<Item = T>>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    options: I,
    value: &mut T,
) -> bool {
    let before = *value;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(value.as_widget_text())
        .show_ui_list(ui, options, value);
    *value != before
}

fn storage_combo_opt_with_none<V: ProjectResource>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    storage: &Storage<V>,
    value: &mut Option<V::Id>,
) -> bool
where
    V::Id: slotmap::Key,
{
    let before = *value;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text_storage_opt(storage, *value)
        .show_ui_storage_opt_with_none(ui, storage, value);
    *value != before
}

pub fn pipeline_entry_ui(
    ui: &mut egui::Ui,
    handle: egui_dnd::Handle<'_>,
    render_pass_id: RenderPassId,
    pipeline_index: usize,
    pipeline: &mut RenderPipeline,
    shaders: &Storage<Shader>,
    bind_groups: &Storage<BindGroup>,
    models: &Storage<Model>,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
    delete_pipeline: &mut Option<usize>,
) {
    let rename_target = RenameTarget::RenderPipeline(render_pass_id, pipeline_index);

    ui.indent("pipeline_body", |ui| {
        handle.ui(ui, |ui| {
            ui.add(renameable_label(
                Label::new(&pipeline.label)
                    .selectable(false)
                    .sense(Sense::click()),
                pending_events,
                rename_state,
                rename_target.clone(),
            ))
            .context_menu(|ui| {
                if ui.button("Rename Pipeline").clicked() {
                    pending_events.push(StateEvent::StartRename(rename_target));
                    ui.close();
                }
                if ui.button("Delete Pipeline").clicked() {
                    *delete_pipeline = Some(pipeline_index);
                    ui.close();
                }
            });
        });

        pipeline_ui(ui, render_pass_id, pipeline, shaders, bind_groups, models);
    });
}

fn pipeline_ui(
    ui: &mut egui::Ui,
    render_pass_id: RenderPassId,
    pipeline: &mut RenderPipeline,
    shaders: &Storage<Shader>,
    bind_groups: &Storage<BindGroup>,
    models: &Storage<Model>,
) {
    CollapsingHeader::new("Shaders")
        .default_open(true)
        .show(ui, |ui| {
            Grid::new(("pipeline_shaders_grid", render_pass_id))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Vertex Shader");
                    let mut vertex_shader = pipeline.vertex_shader;
                    if storage_combo_opt_with_none(ui, "vertex_shader", shaders, &mut vertex_shader)
                    {
                        pipeline.set_vertex_shader(vertex_shader);
                    }
                    ui.end_row();

                    ui.label("Fragment Shader");
                    let mut fragment_shader = pipeline.fragment_shader;
                    if storage_combo_opt_with_none(
                        ui,
                        "fragment_shader",
                        shaders,
                        &mut fragment_shader,
                    ) {
                        pipeline.set_fragment_shader(fragment_shader);
                    }
                    ui.end_row();
                });
        });

    ui.add_space(4.0);

    CollapsingHeader::new("Primitive State")
        .default_open(true)
        .show(ui, |ui| {
            let mut ps = pipeline.primitive_state;
            let before = ps;

            Grid::new(("pipeline_primitive_grid", render_pass_id))
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Topology");
                    combo_list(ui, "topology", TOPOLOGIES, &mut ps.topology);
                    ui.end_row();

                    ui.label("Front Face");
                    combo_list(ui, "front_face", FRONT_FACES, &mut ps.front_face);
                    ui.end_row();

                    ui.label("Cull Mode");
                    combo_list(ui, "cull_mode", CULL_MODES, &mut ps.cull_mode);
                    ui.end_row();

                    ui.label("Polygon Mode");
                    combo_list(ui, "polygon_mode", POLYGON_MODES, &mut ps.polygon_mode);
                    ui.end_row();
                });

            if ps != before {
                pipeline.set_primitive_state(ps);
            }
        });

    ui.add_space(4.0);

    super::bind_group_draw::bind_groups_ui(ui, render_pass_id, pipeline, bind_groups);

    ui.add_space(4.0);

    super::bind_group_draw::draw_ui(ui, render_pass_id, pipeline, models);
}
