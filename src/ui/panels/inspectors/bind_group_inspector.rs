use egui::RichText;

use crate::{
    project::{
        BindGroupId, SamplerId, TextureViewId, UniformId,
        bindgroup::{BindGroupEntry, BindGroupResource},
    },
    state::StateEvent,
    ui::{
        components::{
            hint::hint,
            selector::{selectable_value, selectable_value_storage},
        },
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn bind_group_inspector_ui(&mut self, bind_group_id: BindGroupId, ui: &mut egui::Ui) {
        let entries = match self.project.bind_groups.get_mut(bind_group_id) {
            Some(bg) => bg.entries().to_vec(),
            None => {
                ui.label("Bind group not found");
                return;
            }
        };

        if entries.is_empty() {
            ui.label("No entries in bind group");
        }

        let response =
            egui_dnd::dnd(ui, "bind_group").show(entries.iter(), |ui, item, handle, state| {
                handle.show_drag_cursor_on_hover(false).ui(ui, |ui| {
                    ui.push_id(state.index, |ui| {
                        self.ui_entry(ui, bind_group_id, state.index, item);
                    });
                });
            });

        if let Some(update) = response.final_update() {
            self.pending_events
                .push(StateEvent::ReorderBindGroupEntry(bind_group_id, update));
        }

        ui.add_space(6.0);

        ui.menu_button("Add Entry", |ui| {
            for kind in ResourceKind::ALL {
                if ui.button(kind.label()).clicked() {
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
                ui.label("Drag bindings to reorder them. Right-click a");
                ui.label(RichText::new("Binding").strong());
                ui.label("to remove it.");
            }));
        }
    }

    fn ui_entry(
        &mut self,
        ui: &mut egui::Ui,
        bind_group_id: BindGroupId,
        index: usize,
        entry: &BindGroupEntry,
    ) {
        let mut current_kind: ResourceKind = entry.resource.into();
        let kind_before = current_kind;

        ui.horizontal(|ui| {
            ui.add(
                egui::Label::new(format!("Binding {index}"))
                    .selectable(false)
                    .sense(egui::Sense::click()),
            )
            .context_menu(|ui| {
                if ui.button("Delete Entry").clicked() {
                    self.pending_events
                        .push(StateEvent::DeleteBindGroupEntry(bind_group_id, index));
                    ui.close();
                }
            });

            egui::ComboBox::from_id_salt("kind")
                .selected_text(current_kind.label())
                .show_ui(ui, |ui| {
                    for kind in ResourceKind::ALL {
                        ui.selectable_value(&mut current_kind, kind, kind.label());
                    }
                });
        });

        if current_kind != kind_before {
            let resource = current_kind.default_value();
            self.pending_events.push(StateEvent::UpdateBindGroupEntry(
                bind_group_id,
                index,
                resource,
            ));
        }

        egui::Grid::new("entry_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                self.ui_entry_fields(ui, bind_group_id, index, entry)
            });
    }

    fn ui_entry_fields(
        &mut self,
        ui: &mut egui::Ui,
        bind_group_id: BindGroupId,
        index: usize,
        entry: &BindGroupEntry,
    ) {
        let resource = match entry.resource {
            BindGroupResource::Uniform(id) => self.ui_uniform_fields(ui, id),
            BindGroupResource::Texture {
                texture_view_id,
                view_dimension,
                sample_type,
            } => self.ui_texture_fields(ui, texture_view_id, view_dimension, sample_type),
            BindGroupResource::Sampler {
                sampler_id,
                sampler_binding_type,
            } => self.ui_sampler_fields(ui, sampler_id, sampler_binding_type),
        };

        if let Some(r) = resource {
            self.pending_events
                .push(StateEvent::UpdateBindGroupEntry(bind_group_id, index, r));
        }
    }

    fn ui_uniform_fields(
        &mut self,
        ui: &mut egui::Ui,
        mut uniform_id: Option<UniformId>,
    ) -> Option<BindGroupResource> {
        let before = uniform_id;
        ui.label("Uniform");
        selectable_value_storage(
            ui,
            "resource",
            &mut uniform_id,
            |_, uniform| &uniform.label,
            &self.project.uniforms,
        );
        ui.end_row();
        (uniform_id != before).then_some(BindGroupResource::Uniform(uniform_id))
    }

    fn ui_texture_fields(
        &mut self,
        ui: &mut egui::Ui,
        mut texture_view_id: Option<TextureViewId>,
        mut view_dimension: wgpu::TextureViewDimension,
        mut sample_type: wgpu::TextureSampleType,
    ) -> Option<BindGroupResource> {
        let (tvid_before, vd_before, st_before) = (texture_view_id, view_dimension, sample_type);

        ui.label("Texture View");
        selectable_value_storage(
            ui,
            "resource",
            &mut texture_view_id,
            |_, texture_view| texture_view.label(),
            &self.project.texture_views,
        );
        ui.end_row();

        ui.label("Dimension");
        selectable_value(
            ui,
            "view_dimension",
            &mut view_dimension,
            texture_view_dimension_label,
            TEXTURE_VIEW_DIMENSIONS,
        );
        ui.end_row();

        ui.label("Sample Type");
        selectable_value(
            ui,
            "sample_type",
            &mut sample_type,
            texture_sample_type_label,
            TEXTURE_SAMPLE_TYPES,
        );
        ui.end_row();

        (texture_view_id != tvid_before || view_dimension != vd_before || sample_type != st_before)
            .then_some(BindGroupResource::Texture {
                texture_view_id,
                view_dimension,
                sample_type,
            })
    }

    fn ui_sampler_fields(
        &mut self,
        ui: &mut egui::Ui,
        mut sampler_id: Option<SamplerId>,
        mut sampler_binding_type: wgpu::SamplerBindingType,
    ) -> Option<BindGroupResource> {
        let (sid_before, sbt_before) = (sampler_id, sampler_binding_type);

        ui.label("Sampler");
        selectable_value_storage(
            ui,
            "resource",
            &mut sampler_id,
            |_, sampler| sampler.label(),
            &self.project.samplers,
        );
        ui.end_row();

        ui.label("Binding Type");
        selectable_value(
            ui,
            "sampler_binding_type",
            &mut sampler_binding_type,
            sampler_binding_type_label,
            SAMPLER_BINDING_TYPES,
        );
        ui.end_row();

        (sampler_id != sid_before || sampler_binding_type != sbt_before).then_some(
            BindGroupResource::Sampler {
                sampler_id,
                sampler_binding_type,
            },
        )
    }
}

const TEXTURE_VIEW_DIMENSIONS: [wgpu::TextureViewDimension; 6] = [
    wgpu::TextureViewDimension::D1,
    wgpu::TextureViewDimension::D2,
    wgpu::TextureViewDimension::D2Array,
    wgpu::TextureViewDimension::Cube,
    wgpu::TextureViewDimension::CubeArray,
    wgpu::TextureViewDimension::D3,
];

fn texture_view_dimension_label(dimension: wgpu::TextureViewDimension) -> &'static str {
    match dimension {
        wgpu::TextureViewDimension::D1 => "1D",
        wgpu::TextureViewDimension::D2 => "2D",
        wgpu::TextureViewDimension::D2Array => "2D Array",
        wgpu::TextureViewDimension::Cube => "Cube",
        wgpu::TextureViewDimension::CubeArray => "Cube Array",
        wgpu::TextureViewDimension::D3 => "3D",
    }
}

const TEXTURE_SAMPLE_TYPES: [wgpu::TextureSampleType; 5] = [
    wgpu::TextureSampleType::Float { filterable: true },
    wgpu::TextureSampleType::Float { filterable: false },
    wgpu::TextureSampleType::Depth,
    wgpu::TextureSampleType::Sint,
    wgpu::TextureSampleType::Uint,
];

fn texture_sample_type_label(sample_type: wgpu::TextureSampleType) -> &'static str {
    match sample_type {
        wgpu::TextureSampleType::Float { filterable: true } => "Float (Filterable)",
        wgpu::TextureSampleType::Float { filterable: false } => "Float (Non-Filterable)",
        wgpu::TextureSampleType::Depth => "Depth",
        wgpu::TextureSampleType::Sint => "Sint",
        wgpu::TextureSampleType::Uint => "Uint",
    }
}

const SAMPLER_BINDING_TYPES: [wgpu::SamplerBindingType; 3] = [
    wgpu::SamplerBindingType::Filtering,
    wgpu::SamplerBindingType::NonFiltering,
    wgpu::SamplerBindingType::Comparison,
];

fn sampler_binding_type_label(binding_type: wgpu::SamplerBindingType) -> &'static str {
    match binding_type {
        wgpu::SamplerBindingType::Filtering => "Filtering",
        wgpu::SamplerBindingType::NonFiltering => "Non-Filtering",
        wgpu::SamplerBindingType::Comparison => "Comparison",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    Uniform,
    Texture,
    Sampler,
}

impl ResourceKind {
    const ALL: [ResourceKind; 3] = [
        ResourceKind::Uniform,
        ResourceKind::Texture,
        ResourceKind::Sampler,
    ];

    fn label(self) -> &'static str {
        match self {
            ResourceKind::Uniform => "Uniform",
            ResourceKind::Texture => "Texture",
            ResourceKind::Sampler => "Sampler",
        }
    }

    fn default_value(self) -> BindGroupResource {
        match self {
            ResourceKind::Uniform => BindGroupResource::Uniform(None),
            ResourceKind::Texture => BindGroupResource::Texture {
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

impl From<BindGroupResource> for ResourceKind {
    fn from(resource: BindGroupResource) -> Self {
        match resource {
            BindGroupResource::Uniform(_) => ResourceKind::Uniform,
            BindGroupResource::Texture { .. } => ResourceKind::Texture,
            BindGroupResource::Sampler { .. } => ResourceKind::Sampler,
        }
    }
}
