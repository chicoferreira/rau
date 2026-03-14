use crate::{
    project::{
        BindGroupId, SamplerId, TextureViewId, UniformId,
        bindgroup::{BindGroupEntry, BindGroupResource},
    },
    state::StateEvent,
    ui::{components::selector::combo_grid_row, pane::StateSnapshot},
};

const TEXTURE_VIEW_DIMENSIONS: &[(wgpu::TextureViewDimension, &str)] = &[
    (wgpu::TextureViewDimension::D1, "1D"),
    (wgpu::TextureViewDimension::D2, "2D"),
    (wgpu::TextureViewDimension::D2Array, "2D Array"),
    (wgpu::TextureViewDimension::Cube, "Cube"),
    (wgpu::TextureViewDimension::CubeArray, "Cube Array"),
    (wgpu::TextureViewDimension::D3, "3D"),
];

const TEXTURE_SAMPLE_TYPES: &[(wgpu::TextureSampleType, &str)] = &[
    (
        wgpu::TextureSampleType::Float { filterable: true },
        "Float (Filterable)",
    ),
    (
        wgpu::TextureSampleType::Float { filterable: false },
        "Float (Non-Filterable)",
    ),
    (wgpu::TextureSampleType::Depth, "Depth"),
    (wgpu::TextureSampleType::Sint, "Sint"),
    (wgpu::TextureSampleType::Uint, "Uint"),
];

const SAMPLER_BINDING_TYPES: &[(wgpu::SamplerBindingType, &str)] = &[
    (wgpu::SamplerBindingType::Filtering, "Filtering"),
    (wgpu::SamplerBindingType::NonFiltering, "Non-Filtering"),
    (wgpu::SamplerBindingType::Comparison, "Comparison"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceKind {
    Uniform,
    Texture,
    Sampler,
}

impl ResourceKind {
    fn label(self) -> &'static str {
        match self {
            ResourceKind::Uniform => "Uniform",
            ResourceKind::Texture => "Texture",
            ResourceKind::Sampler => "Sampler",
        }
    }
}

fn resource_kind(resource: &BindGroupResource) -> ResourceKind {
    match resource {
        BindGroupResource::Uniform(_) => ResourceKind::Uniform,
        BindGroupResource::Texture { .. } => ResourceKind::Texture,
        BindGroupResource::Sampler { .. } => ResourceKind::Sampler,
    }
}

impl StateSnapshot<'_> {
    pub fn bind_group_inspector_ui(&mut self, bind_group_id: BindGroupId, ui: &mut egui::Ui) {
        let entries = match self.project.bind_groups.get(bind_group_id) {
            Some(bg) => bg.entries().to_vec(),
            None => {
                ui.label("Bind group not found.");
                return;
            }
        };

        let uniforms: Vec<(UniformId, &str)> = self
            .project
            .uniforms
            .list()
            .map(|(id, u)| (id, u.label.as_str()))
            .collect();

        let texture_views: Vec<(TextureViewId, &str)> = self
            .project
            .texture_views
            .list()
            .map(|(id, tv)| (id, tv.label()))
            .collect();

        let samplers: Vec<(SamplerId, &str)> = self
            .project
            .samplers
            .list()
            .map(|(id, s)| (id, s.label.as_ref()))
            .collect();

        ui.add_space(4.0);

        for (index, entry) in entries.iter().enumerate() {
            if index != 0 {
                ui.separator();
            }

            ui.push_id(index, |ui| {
                if let Some(event) = ui_entry(
                    ui,
                    bind_group_id,
                    index,
                    entry,
                    &uniforms,
                    &texture_views,
                    &samplers,
                ) {
                    self.pending_events.push(event);
                }
            });
        }

        ui.add_space(6.0);

        if let Some(event) =
            ui_add_entry_menu(ui, bind_group_id, &uniforms, &texture_views, &samplers)
        {
            self.pending_events.push(event);
        }
    }
}

fn ui_entry(
    ui: &mut egui::Ui,
    bind_group_id: BindGroupId,
    index: usize,
    entry: &BindGroupEntry,
    uniforms: &[(UniformId, &str)],
    texture_views: &[(TextureViewId, &str)],
    samplers: &[(SamplerId, &str)],
) -> Option<StateEvent> {
    let mut delete = false;
    let mut kind = resource_kind(&entry.resource);
    let kind_before = kind;

    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(egui::RichText::new(format!("Binding {index}")).strong())
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Delete Entry").clicked() {
                delete = true;
                ui.close();
            }
        });

        egui::ComboBox::from_id_salt("kind")
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut kind, ResourceKind::Uniform, "Uniform");
                ui.selectable_value(&mut kind, ResourceKind::Texture, "Texture");
                ui.selectable_value(&mut kind, ResourceKind::Sampler, "Sampler");
            });
    });

    if delete {
        return Some(StateEvent::DeleteBindGroupEntry(bind_group_id, index));
    }

    if kind != kind_before {
        let updated = match kind {
            ResourceKind::Uniform => uniforms
                .first()
                .map(|(id, _)| BindGroupResource::Uniform(*id)),
            ResourceKind::Texture => {
                texture_views
                    .first()
                    .map(|(id, _)| BindGroupResource::Texture {
                        texture_view_id: *id,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    })
            }
            ResourceKind::Sampler => samplers.first().map(|(id, _)| BindGroupResource::Sampler {
                sampler_id: *id,
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        };
        return updated
            .map(|resource| StateEvent::UpdateBindGroupEntry(bind_group_id, index, resource));
    }

    egui::Grid::new("entry_grid")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui_entry_grid(
                ui,
                bind_group_id,
                index,
                entry,
                uniforms,
                texture_views,
                samplers,
            )
        })
        .inner
}

fn ui_entry_grid(
    ui: &mut egui::Ui,
    bind_group_id: BindGroupId,
    index: usize,
    entry: &BindGroupEntry,
    uniforms: &[(UniformId, &str)],
    texture_views: &[(TextureViewId, &str)],
    samplers: &[(SamplerId, &str)],
) -> Option<StateEvent> {
    let resource = match entry.resource {
        BindGroupResource::Uniform(id) => ui_uniform_rows(ui, id, uniforms),
        BindGroupResource::Texture {
            texture_view_id,
            view_dimension,
            sample_type,
        } => ui_texture_rows(
            ui,
            texture_view_id,
            view_dimension,
            sample_type,
            texture_views,
        ),
        BindGroupResource::Sampler {
            sampler_id,
            sampler_binding_type,
        } => ui_sampler_rows(ui, sampler_id, sampler_binding_type, samplers),
    };
    resource.map(|r| StateEvent::UpdateBindGroupEntry(bind_group_id, index, r))
}

fn ui_uniform_rows(
    ui: &mut egui::Ui,
    mut uniform_id: UniformId,
    uniforms: &[(UniformId, &str)],
) -> Option<BindGroupResource> {
    let before = uniform_id;
    ui.label("Uniform");
    combo_grid_row(
        ui,
        "resource",
        &mut uniform_id,
        uniforms,
        "No uniforms available",
    );
    ui.end_row();
    (uniform_id != before).then_some(BindGroupResource::Uniform(uniform_id))
}

fn ui_texture_rows(
    ui: &mut egui::Ui,
    mut texture_view_id: TextureViewId,
    mut view_dimension: wgpu::TextureViewDimension,
    mut sample_type: wgpu::TextureSampleType,
    texture_views: &[(TextureViewId, &str)],
) -> Option<BindGroupResource> {
    let (tvid_before, vd_before, st_before) = (texture_view_id, view_dimension, sample_type);

    ui.label("Texture View");
    combo_grid_row(
        ui,
        "resource",
        &mut texture_view_id,
        texture_views,
        "No texture views available",
    );
    ui.end_row();

    ui.label("Dimension");
    combo_grid_row(
        ui,
        "view_dimension",
        &mut view_dimension,
        TEXTURE_VIEW_DIMENSIONS,
        "",
    );
    ui.end_row();

    ui.label("Sample Type");
    combo_grid_row(
        ui,
        "sample_type",
        &mut sample_type,
        TEXTURE_SAMPLE_TYPES,
        "",
    );
    ui.end_row();

    (texture_view_id != tvid_before || view_dimension != vd_before || sample_type != st_before)
        .then_some(BindGroupResource::Texture {
            texture_view_id,
            view_dimension,
            sample_type,
        })
}

fn ui_sampler_rows(
    ui: &mut egui::Ui,
    mut sampler_id: SamplerId,
    mut sampler_binding_type: wgpu::SamplerBindingType,
    samplers: &[(SamplerId, &str)],
) -> Option<BindGroupResource> {
    let (sid_before, sbt_before) = (sampler_id, sampler_binding_type);

    ui.label("Sampler");
    combo_grid_row(
        ui,
        "resource",
        &mut sampler_id,
        samplers,
        "No samplers available",
    );
    ui.end_row();

    ui.label("Binding Type");
    combo_grid_row(
        ui,
        "sampler_binding_type",
        &mut sampler_binding_type,
        SAMPLER_BINDING_TYPES,
        "",
    );
    ui.end_row();

    (sampler_id != sid_before || sampler_binding_type != sbt_before).then_some(
        BindGroupResource::Sampler {
            sampler_id,
            sampler_binding_type,
        },
    )
}

/// Renders a single "Add X" submenu inside an existing menu.
/// Returns the chosen resource, if any button was clicked.
fn ui_add_entry_submenu<Id: Copy>(
    ui: &mut egui::Ui,
    title: &str,
    options: &[(Id, &str)],
    empty_msg: &str,
    make_resource: impl Fn(Id) -> BindGroupResource,
) -> Option<BindGroupResource> {
    let mut result = None;
    ui.menu_button(title, |ui| {
        if options.is_empty() {
            ui.label(egui::RichText::new(empty_msg).weak());
        }
        for (id, label) in options {
            if ui.button(*label).clicked() {
                result = Some(make_resource(*id));
                ui.close();
            }
        }
    });
    result
}

fn ui_add_entry_menu(
    ui: &mut egui::Ui,
    bind_group_id: BindGroupId,
    uniforms: &[(UniformId, &str)],
    texture_views: &[(TextureViewId, &str)],
    samplers: &[(SamplerId, &str)],
) -> Option<StateEvent> {
    let mut result = None;

    ui.menu_button("Add Entry", |ui| {
        let uniform =
            ui_add_entry_submenu(ui, "Uniform", uniforms, "No uniforms available", |id| {
                BindGroupResource::Uniform(id)
            });
        let texture = ui_add_entry_submenu(
            ui,
            "Texture",
            texture_views,
            "No texture views available",
            |id| BindGroupResource::Texture {
                texture_view_id: id,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            },
        );
        let sampler =
            ui_add_entry_submenu(ui, "Sampler", samplers, "No samplers available", |id| {
                BindGroupResource::Sampler {
                    sampler_id: id,
                    sampler_binding_type: wgpu::SamplerBindingType::Filtering,
                }
            });
        result = uniform.or(texture).or(sampler);
    });

    result.map(|r| StateEvent::CreateBindGroupEntry(bind_group_id, r))
}
