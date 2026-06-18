use std::ops::Range;

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
            field_docs::field_doc,
            inspector::{self, AsWidgetText},
            resource_icons,
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
            inspector::row_doc(
                ui,
                "Vertex Shader",
                field_doc!(
                    "The shader that positions each vertex (the **vertex stage**).\n\n\
                    WGSL marks the entry point with `@vertex`; GLSL uses `void main()` in a \
                    `.vert` file.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpuvertexstate)"
                ),
                |ui| {
                    inspector::storage_combo(
                        ui,
                        "render_pipeline_vertex_shader",
                        shaders,
                        &mut vertex_shader,
                    )
                },
            );
            inspector::row_doc(
                ui,
                "Fragment Shader",
                field_doc!(
                    "The shader that computes each pixel's color (the **fragment stage**).\n\n\
                    WGSL marks the entry point with `@fragment`; GLSL uses `void main()` in a \
                    `.frag` file.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpufragmentstate)"
                ),
                |ui| {
                    inspector::storage_combo(
                        ui,
                        "render_pipeline_fragment_shader",
                        shaders,
                        &mut fragment_shader,
                    )
                },
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
            inspector::combo_row_doc(
                ui,
                "Color Format",
                field_doc!(
                    "Pixel format of the color attachment this pipeline writes to. Must match \
                    the Render Pass's color target.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpucolortargetstate)"
                ),
                "render_pipeline_color_format",
                TextureFormat::COLOR,
                &mut color_format,
            );

            let mut depth_enabled = depth_format.is_some();
            if inspector::checkbox_row_doc(
                ui,
                "Depth",
                field_doc!("Whether this pipeline performs depth testing and writes depth."),
                &mut depth_enabled,
            ) {
                depth_format = depth_enabled.then_some(TextureFormat::Depth32Float);
            }

            if let Some(format) = &mut depth_format {
                inspector::combo_row_doc(
                    ui,
                    "Depth Format",
                    field_doc!(
                        "Pixel format of the depth attachment. Must match the Render Pass's \
                        depth target.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpudepthstencilstate)"
                    ),
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

    inspector::section_doc(
        ui,
        "Primitive State",
        field_doc!(
            "How vertices are assembled into primitives and rasterized.\n\n\
            [WebGPU spec](https://www.w3.org/TR/webgpu/#dictdef-gpuprimitivestate)"
        ),
        |ui| {
            inspector::field_grid(ui, (render_pipeline_id, "primitive_state"), |ui| {
                inspector::combo_row_doc(
                    ui,
                    "Topology",
                    field_doc!(
                        "How vertices are grouped into primitives: point, line, or triangle lists \
                        and strips.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpuprimitivetopology)"
                    ),
                    "render_pipeline_topology",
                    TOPOLOGIES,
                    &mut primitive_state.topology,
                );
                inspector::combo_row_doc(
                    ui,
                    "Strip Index Format",
                    field_doc!(
                        "For **strip** topologies, the index value that restarts the strip. Leave \
                    as **None** for list topologies.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuprimitivestate-stripindexformat)"
                    ),
                    "render_pipeline_strip_index_format",
                    STRIP_INDEX_FORMATS,
                    &mut primitive_state.strip_index_format,
                );
                inspector::combo_row_doc(
                    ui,
                    "Front Face",
                    field_doc!(
                        "Which winding order counts as the **front** face: counter-clockwise or \
                    clockwise.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuprimitivestate-frontface)"
                    ),
                    "render_pipeline_front_face",
                    FRONT_FACES,
                    &mut primitive_state.front_face,
                );
                inspector::combo_row_doc(
                    ui,
                    "Cull Mode",
                    field_doc!(
                        "Which faces are discarded before rasterization: front, back, or none.\n\n\
                    [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuprimitivestate-cullmode)"
                    ),
                    "render_pipeline_cull_mode",
                    CULL_MODES,
                    &mut primitive_state.cull_mode,
                );
                inspector::combo_row_doc(
                    ui,
                    "Polygon Mode",
                    field_doc!(
                        "Whether polygons are **filled** or drawn as lines/points (wireframe)."
                    ),
                    "render_pipeline_polygon_mode",
                    POLYGON_MODES,
                    &mut primitive_state.polygon_mode,
                );
                inspector::checkbox_row_doc(
                    ui,
                    "Unclipped Depth",
                    field_doc!(
                        "Clamp fragments beyond the near/far planes instead of clipping them. \
                        Requires the `DEPTH_CLIP_CONTROL` feature."
                    ),
                    &mut primitive_state.unclipped_depth,
                );
                inspector::checkbox_row_doc(
                    ui,
                    "Conservative",
                    field_doc!(
                        "Enable conservative rasterization: a primitive covers a pixel if it \
                        touches it at all. Requires the `CONSERVATIVE_RASTERIZATION` feature."
                    ),
                    &mut primitive_state.conservative,
                );
            });
        },
    );

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

    inspector::section_doc(
        ui,
        &format!("Bind Groups ({})", entries.len()),
        field_doc!(
            "The Bind Group bound at each slot (`@group(n)`) while this pipeline draws.\n\n\
            - **Empty**: nothing bound.\n\
            - **Static**: a fixed Bind Group resource.\n\
            - **Model Material**: uses each mesh's own material bind group (Model draw only).\n\n\
            Drag to reorder, right-click to remove.\n\n\
            [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpurenderpassencoder-setbindgroup)"
        ),
        |ui| {
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

            edits.apply(&mut entries);

            if entries != before {
                render_pipeline.set_bind_groups(entries);
            }
        },
    );
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
    let bind_group_text = |ui: &egui::Ui, id: crate::project::BindGroupId, label: &str| {
        resource_icons::icon_text(ui, resource_icons::resource_id_icon(id.into()), label)
    };

    let selected_text: egui::WidgetText = match target {
        BindGroupTarget::Empty => "Empty".into(),
        BindGroupTarget::ModelMaterial => "Model Material".into(),
        BindGroupTarget::Static(id) => match bind_groups.get_label(*id) {
            Ok(label) => bind_group_text(ui, *id, label),
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
                let text = bind_group_text(ui, id, bind_group.label());
                if ui.selectable_label(selected, text).clicked() {
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

    inspector::section_doc_wide(
        ui,
        "Draw",
        field_doc!(
            r"How this pipeline issues its draw call when run by a Render Pass step.

Roughly:

```rs
set_pipeline(pipeline)
for slot, bind_group in bind_groups:
  set_bind_group(slot, bind_group)
if draw kind is Model:
  for mesh in model.meshes:
    set_vertex_buffer(mesh_vertex_slot, mesh.vertices)
    set_index_buffer(mesh.indices)
    // 'Model Material' slots use the mesh's own bind group:
    for slot where target == Model Material:
      set_bind_group(slot, mesh.material.bind_group)
    draw_indexed(0..mesh.index_count, instances)
else: // Direct draw
  draw(vertices, instances)
```

[WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpurenderpassencoder-draw)"
        ),
        |ui| {
            inspector::field_grid(ui, (render_pipeline_id, "draw_strategy"), |ui| {
                let mut draw_kind = DrawKind::from_strategy(&edited);
                if inspector::combo_row_doc(
                    ui,
                    "Draw Kind",
                    field_doc!(
                        "How vertices are sourced:\n\n\
                    - **Direct**: draw a fixed range of vertices with no vertex/index buffer \
                    (e.g. a full-screen triangle).\n\
                    - **Model**: draw each mesh of a Model using its vertex and index buffers."
                    ),
                    "render_pipeline_draw_kind",
                    DRAW_KINDS,
                    &mut draw_kind,
                ) {
                    edited = draw_kind.default_strategy();
                }

                draw_strategy_fields_ui(ui, models, &mut edited);
            });
        },
    );

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
            inspector::row_doc(
                ui,
                "Vertices",
                field_doc!("Range of vertex indices to draw, e.g. `0..3` for a single triangle."),
                |ui| range_u32_edit(ui, vertices),
            );
            inspector::row_doc(
                ui,
                "Instances",
                field_doc!(
                    "Range of instance indices to draw. `0..1` draws once; widen it for \
                    instanced rendering."
                ),
                |ui| range_u32_edit(ui, instances),
            );
        }
        RenderDrawStrategy::Model {
            model_id,
            instances,
            mesh_vertex_slot,
        } => {
            inspector::row_doc(
                ui,
                "Model",
                field_doc!("The Model whose meshes are drawn, one draw call per mesh."),
                |ui| inspector::storage_combo(ui, "render_pipeline_model", models, model_id),
            );
            inspector::row_doc(
                ui,
                "Instances",
                field_doc!(
                    "Range of instance indices to draw. `0..1` draws once; widen it for \
                    instanced rendering."
                ),
                |ui| range_u32_edit(ui, instances),
            );
            inspector::u32_drag_row_doc(
                ui,
                "Mesh Vertex Slot",
                field_doc!("The vertex buffer slot the mesh's vertices are bound to."),
                mesh_vertex_slot,
                0..=u32::MAX,
            );
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
