use egui::RichText;
use egui_dnd::DragDropItem;
use strum::IntoEnumIterator;

use crate::{
    project::{
        BindGroupId, SamplerId, TextureViewId, UniformId,
        bindgroup::{BindGroupEntry, BindGroupResource},
        sampler::Sampler,
        storage::Storage,
        texture_view::TextureView,
        uniform::Uniform,
    },
    state::StateEvent,
    ui::{
        components::{
            hint::hint,
            selector::{AsWidgetText, ComboBoxExt},
        },
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn bind_group_inspector_ui(&mut self, bind_group_id: BindGroupId, ui: &mut egui::Ui) {
        let Ok(bind_group) = self.project.bind_groups.get(bind_group_id) else {
            ui.label("Bind group not found");
            return;
        };

        let entries = bind_group.entries();
        if entries.is_empty() {
            ui.label("No entries in bind group");
        }

        let mut ctx = BindGroupUiContext {
            pending_events: self.pending_events,
            uniforms: &self.project.uniforms,
            texture_views: &self.project.texture_views,
            samplers: &self.project.samplers,
        };

        let response = egui_dnd::dnd(ui, "bind_group").show_custom(|ui, iter| {
            for (index, field) in entries.iter().enumerate() {
                if index != 0 {
                    ui.add_space(5.0);
                }
                ui.push_id(index, |ui| {
                    iter.next(ui, field.id(), index, true, |ui, item_handle| {
                        item_handle.ui(ui, |ui, handle, _state| {
                            handle.ui(ui, |ui| {
                                ui_entry_title(ui, &mut ctx, bind_group_id, index);
                            });
                            ui_entry_fields(ui, &mut ctx, bind_group_id, index, field);
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            self.pending_events
                .push(StateEvent::ReorderBindGroupEntry(bind_group_id, update));
        }

        ui.add_space(6.0);

        ui.menu_button("Add Entry", |ui| {
            for kind in ResourceKind::iter() {
                if ui.button(kind.to_string()).clicked() {
                    ui.close();
                    let resource = kind.default_value();
                    self.pending_events
                        .push(StateEvent::CreateBindGroupEntry(bind_group_id, resource));
                }
            }
        });

        if !entries.is_empty() {
            ui.add_space(6.0);
            ui.add(hint(|ui| {
                ui.label("Right-click a");
                ui.label(RichText::new("Binding").strong());
                ui.label("to remove it or drag it to reorder it.");
            }));
        }
    }
}

struct BindGroupUiContext<'a> {
    pending_events: &'a mut Vec<StateEvent>,
    uniforms: &'a Storage<UniformId, Uniform>,
    texture_views: &'a Storage<TextureViewId, TextureView>,
    samplers: &'a Storage<SamplerId, Sampler>,
}

fn ui_entry_title(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    bind_group_id: BindGroupId,
    index: usize,
) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(format!("Binding {index}"))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Delete Entry").clicked() {
                ctx.pending_events
                    .push(StateEvent::DeleteBindGroupEntry(bind_group_id, index));
                ui.close();
            }
        });
    });
}

fn ui_entry_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    bind_group_id: BindGroupId,
    index: usize,
    entry: &BindGroupEntry,
) {
    ui.vertical(|ui| {
        ui.indent("entry", |ui| {
            egui::Grid::new("entry_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    let mut current_kind: ResourceKind = entry.resource.into();
                    let kind_before = current_kind;
                    ui.label("Resource");
                    egui::ComboBox::from_id_salt("resource")
                        .selected_text(current_kind.as_widget_text())
                        .show_ui_list(ui, ResourceKind::iter(), &mut current_kind);
                    ui.end_row();

                    let resource_from_fields = match entry.resource {
                        BindGroupResource::Uniform(id) => ui_uniform_fields(ui, ctx, id),
                        BindGroupResource::Texture {
                            texture_view_id,
                            view_dimension,
                            sample_type,
                        } => {
                            ui_texture_fields(ui, ctx, texture_view_id, view_dimension, sample_type)
                        }
                        BindGroupResource::Sampler {
                            sampler_id,
                            sampler_binding_type,
                        } => ui_sampler_fields(ui, ctx, sampler_id, sampler_binding_type),
                    };

                    let resource = (current_kind != kind_before)
                        .then_some(current_kind.default_value())
                        .or(resource_from_fields);

                    if let Some(r) = resource {
                        ctx.pending_events.push(StateEvent::UpdateBindGroupEntry(
                            bind_group_id,
                            index,
                            r,
                        ));
                    }
                });
        });
    });
}

fn ui_uniform_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    mut uniform_id: Option<UniformId>,
) -> Option<BindGroupResource> {
    let before = uniform_id;
    ui.label("Uniform");
    egui::ComboBox::from_id_salt("uniform")
        .selected_text_storage_opt(ctx.uniforms, uniform_id)
        .show_ui_storage_opt(ui, ctx.uniforms, &mut uniform_id);
    ui.end_row();
    (uniform_id != before).then_some(BindGroupResource::Uniform(uniform_id))
}

fn ui_texture_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    mut texture_view_id: Option<TextureViewId>,
    mut view_dimension: wgpu::TextureViewDimension,
    mut sample_type: wgpu::TextureSampleType,
) -> Option<BindGroupResource> {
    let (tvid_before, vd_before, st_before) = (texture_view_id, view_dimension, sample_type);

    ui.label("Texture View");
    egui::ComboBox::from_id_salt("texture view")
        .selected_text_storage_opt(ctx.texture_views, texture_view_id)
        .show_ui_storage_opt(ui, ctx.texture_views, &mut texture_view_id);

    ui.end_row();

    ui.label("Dimension");
    egui::ComboBox::from_id_salt("view_dimension")
        .selected_text(view_dimension.as_widget_text())
        .show_ui_list(ui, TEXTURE_VIEW_DIMENSIONS, &mut view_dimension);

    ui.end_row();

    ui.label("Sample Type");
    egui::ComboBox::from_id_salt("sample_type")
        .selected_text(sample_type.as_widget_text())
        .show_ui_list(ui, TEXTURE_SAMPLE_TYPES, &mut sample_type);
    ui.end_row();

    (texture_view_id != tvid_before || view_dimension != vd_before || sample_type != st_before)
        .then_some(BindGroupResource::Texture {
            texture_view_id,
            view_dimension,
            sample_type,
        })
}

fn ui_sampler_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    mut sampler_id: Option<SamplerId>,
    mut sampler_binding_type: wgpu::SamplerBindingType,
) -> Option<BindGroupResource> {
    let (sid_before, sbt_before) = (sampler_id, sampler_binding_type);

    ui.label("Sampler");
    egui::ComboBox::from_id_salt("sampler")
        .selected_text_storage_opt(ctx.samplers, sampler_id)
        .show_ui_storage_opt(ui, ctx.samplers, &mut sampler_id);
    ui.end_row();

    ui.label("Binding Type");
    egui::ComboBox::from_id_salt("sampler_binding_type")
        .selected_text(sampler_binding_type.as_widget_text())
        .show_ui_list(ui, SAMPLER_BINDING_TYPES, &mut sampler_binding_type);
    ui.end_row();

    (sampler_id != sid_before || sampler_binding_type != sbt_before).then_some(
        BindGroupResource::Sampler {
            sampler_id,
            sampler_binding_type,
        },
    )
}

const TEXTURE_VIEW_DIMENSIONS: [wgpu::TextureViewDimension; 6] = [
    wgpu::TextureViewDimension::D1,
    wgpu::TextureViewDimension::D2,
    wgpu::TextureViewDimension::D2Array,
    wgpu::TextureViewDimension::Cube,
    wgpu::TextureViewDimension::CubeArray,
    wgpu::TextureViewDimension::D3,
];

impl AsWidgetText for wgpu::TextureViewDimension {
    fn as_widget_text(&self) -> egui::WidgetText {
        let s = match self {
            wgpu::TextureViewDimension::D1 => "1D",
            wgpu::TextureViewDimension::D2 => "2D",
            wgpu::TextureViewDimension::D2Array => "2D Array",
            wgpu::TextureViewDimension::Cube => "Cube",
            wgpu::TextureViewDimension::CubeArray => "Cube Array",
            wgpu::TextureViewDimension::D3 => "3D",
        };
        s.into()
    }
}

const TEXTURE_SAMPLE_TYPES: [wgpu::TextureSampleType; 5] = [
    wgpu::TextureSampleType::Float { filterable: true },
    wgpu::TextureSampleType::Float { filterable: false },
    wgpu::TextureSampleType::Depth,
    wgpu::TextureSampleType::Sint,
    wgpu::TextureSampleType::Uint,
];

impl AsWidgetText for wgpu::TextureSampleType {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::TextureSampleType::Float { filterable: true } => "Float (Filterable)",
            wgpu::TextureSampleType::Float { filterable: false } => "Float (Non-Filterable)",
            wgpu::TextureSampleType::Depth => "Depth",
            wgpu::TextureSampleType::Sint => "Sint",
            wgpu::TextureSampleType::Uint => "Uint",
        };
        r.into()
    }
}

const SAMPLER_BINDING_TYPES: [wgpu::SamplerBindingType; 3] = [
    wgpu::SamplerBindingType::Filtering,
    wgpu::SamplerBindingType::NonFiltering,
    wgpu::SamplerBindingType::Comparison,
];

impl AsWidgetText for wgpu::SamplerBindingType {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::SamplerBindingType::Filtering => "Filtering",
            wgpu::SamplerBindingType::NonFiltering => "Non-Filtering",
            wgpu::SamplerBindingType::Comparison => "Comparison",
        };
        r.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
enum ResourceKind {
    Uniform,
    #[strum(to_string = "Texture View")]
    TextureView,
    Sampler,
}

impl ResourceKind {
    fn default_value(self) -> BindGroupResource {
        match self {
            ResourceKind::Uniform => BindGroupResource::Uniform(None),
            ResourceKind::TextureView => BindGroupResource::Texture {
                texture_view_id: None,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
            ResourceKind::Sampler => BindGroupResource::Sampler {
                sampler_id: None,
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            },
        }
    }
}

impl AsWidgetText for ResourceKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.to_string().into()
    }
}

impl From<BindGroupResource> for ResourceKind {
    fn from(resource: BindGroupResource) -> Self {
        match resource {
            BindGroupResource::Uniform(_) => ResourceKind::Uniform,
            BindGroupResource::Texture { .. } => ResourceKind::TextureView,
            BindGroupResource::Sampler { .. } => ResourceKind::Sampler,
        }
    }
}
