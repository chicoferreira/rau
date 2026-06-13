use egui::RichText;
use strum::IntoEnumIterator;

use crate::{
    project::{
        BindGroupId, SamplerId, TextureViewId, UniformId,
        resource::{
            bindgroup::{BindGroupEntry, BindGroupResource},
            sampler::Sampler,
            texture_view::TextureView,
            uniform::Uniform,
        },
        storage::Storage,
    },
    ui::{
        components::{
            code_editor::shader_code_section,
            draggable_list::{ListEdits, draggable_list},
            flags_selector::flags_selector,
            hint::hint,
            inspector,
            selector::AsWidgetText,
        },
        pane::StateSnapshot,
    },
    utils::shader_preview::{BindGroupAt, ShaderGenCtx},
};

impl StateSnapshot<'_> {
    pub fn bind_group_inspector_ui(&mut self, bind_group_id: BindGroupId, ui: &mut egui::Ui) {
        let Ok(bind_group) = self.project.bind_groups.get_mut(bind_group_id) else {
            ui.label("Bind group not found");
            return;
        };

        let mut entries = bind_group.entries().to_vec();

        let mut ctx = BindGroupUiContext {
            uniforms: &self.project.uniforms,
            texture_views: &self.project.texture_views,
            samplers: &self.project.samplers,
        };

        inspector::section(ui, "Bindings", |ui| {
            if entries.is_empty() {
                ui.label("No entries in bind group");
            }

            let mut edits = draggable_list(
                ui,
                ("bind_group", bind_group_id),
                &entries,
                |ui, field, index, handle, edits| {
                    handle.ui(ui, |ui| {
                        ui.add(
                            egui::Label::new(format!("Binding {index}"))
                                .sense(egui::Sense::click()),
                        )
                        .context_menu(|ui| {
                            if ui.button("Delete Entry").clicked() {
                                edits.push_remove_edit(index);
                                ui.close();
                            }
                        });
                    });
                    ui_entry_fields(ui, &mut ctx, edits, index, field);
                },
            );

            ui.add_space(6.0);

            ui.menu_button("Add Entry", |ui| {
                for kind in ResourceKind::iter() {
                    if ui.button(kind.to_string()).clicked() {
                        ui.close();
                        edits.push_add_edit(BindGroupEntry::new_vertex_fragment(
                            kind.default_value(),
                        ));
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

            edits.apply(&mut entries);

            if bind_group.entries() != entries {
                bind_group.set_entries(entries);
            }
        });

        let Ok(bind_group) = self.project.bind_groups.get(bind_group_id) else {
            return;
        };
        let ctx = ShaderGenCtx::from_project(self.project);
        let item = BindGroupAt::new(None, &bind_group);
        shader_code_section(ui, &item, &ctx);
    }
}

struct BindGroupUiContext<'a> {
    uniforms: &'a Storage<Uniform>,
    texture_views: &'a Storage<TextureView>,
    samplers: &'a Storage<Sampler>,
}

fn ui_entry_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    edits: &mut ListEdits<BindGroupEntry>,
    index: usize,
    entry: &BindGroupEntry,
) {
    ui.vertical(|ui| {
        ui.indent("entry", |ui| {
            inspector::field_grid(ui, "entry_grid", |ui| {
                let mut current_kind: ResourceKind = entry.resource.into();
                let kind_changed = inspector::combo_row(
                    ui,
                    "Resource",
                    "resource",
                    ResourceKind::iter(),
                    &mut current_kind,
                );

                let mut visibility = entry.visibility;
                const SHADER_STAGE_OPTIONS: &[(wgpu::ShaderStages, &str)] = &[
                    (wgpu::ShaderStages::VERTEX, "COPY_SRC"),
                    (wgpu::ShaderStages::FRAGMENT, "COPY_DST"),
                    (wgpu::ShaderStages::COMPUTE, "TEXTURE_BINDING"),
                ];
                inspector::row(ui, "Visibility", |ui| {
                    flags_selector(ui, "visibility", &mut visibility, SHADER_STAGE_OPTIONS);
                });

                let resource_from_fields = match entry.resource {
                    BindGroupResource::Uniform(id) => ui_uniform_fields(ui, ctx, id),
                    BindGroupResource::Texture {
                        texture_view_id,
                        view_dimension,
                        sample_type,
                    } => ui_texture_fields(ui, ctx, texture_view_id, view_dimension, sample_type),
                    BindGroupResource::Sampler {
                        sampler_id,
                        sampler_binding_type,
                    } => ui_sampler_fields(ui, ctx, sampler_id, sampler_binding_type),
                    BindGroupResource::StorageTexture {
                        texture_view_id,
                        access,
                        view_dimension,
                    } => {
                        ui_storage_texture_fields(ui, ctx, texture_view_id, access, view_dimension)
                    }
                };

                let resource = kind_changed
                    .then_some(current_kind.default_value())
                    .or(resource_from_fields);

                let updated_entry = BindGroupEntry {
                    resource: resource.unwrap_or(entry.resource),
                    visibility,
                    ..*entry
                };

                edits.push_set_edit(index, updated_entry);
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
    inspector::storage_opt_combo_row(ui, "Uniform", "uniform", ctx.uniforms, &mut uniform_id);
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

    inspector::storage_opt_combo_row(
        ui,
        "Texture View",
        "texture view",
        ctx.texture_views,
        &mut texture_view_id,
    );
    inspector::combo_row(
        ui,
        "Dimension",
        "view_dimension",
        TEXTURE_VIEW_DIMENSIONS,
        &mut view_dimension,
    );
    inspector::combo_row(
        ui,
        "Sample Type",
        "sample_type",
        TEXTURE_SAMPLE_TYPES,
        &mut sample_type,
    );

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

    inspector::storage_opt_combo_row(ui, "Sampler", "sampler", ctx.samplers, &mut sampler_id);
    inspector::combo_row(
        ui,
        "Binding Type",
        "sampler_binding_type",
        SAMPLER_BINDING_TYPES,
        &mut sampler_binding_type,
    );

    (sampler_id != sid_before || sampler_binding_type != sbt_before).then_some(
        BindGroupResource::Sampler {
            sampler_id,
            sampler_binding_type,
        },
    )
}

fn ui_storage_texture_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    mut texture_view_id: Option<TextureViewId>,
    mut access: wgpu::StorageTextureAccess,
    mut view_dimension: wgpu::TextureViewDimension,
) -> Option<BindGroupResource> {
    let before = (texture_view_id, access, view_dimension);

    inspector::storage_opt_combo_row(
        ui,
        "Texture View",
        "storage_texture_view",
        ctx.texture_views,
        &mut texture_view_id,
    );
    inspector::combo_row(
        ui,
        "Access",
        "storage_texture_access",
        STORAGE_TEXTURE_ACCESS,
        &mut access,
    );
    inspector::combo_row(
        ui,
        "Dimension",
        "storage_texture_view_dimension",
        TEXTURE_VIEW_DIMENSIONS,
        &mut view_dimension,
    );

    ((texture_view_id, access, view_dimension) != before).then_some(
        BindGroupResource::StorageTexture {
            texture_view_id,
            access,
            view_dimension,
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

const STORAGE_TEXTURE_ACCESS: [wgpu::StorageTextureAccess; 4] = [
    wgpu::StorageTextureAccess::WriteOnly,
    wgpu::StorageTextureAccess::ReadOnly,
    wgpu::StorageTextureAccess::ReadWrite,
    wgpu::StorageTextureAccess::Atomic,
];

impl AsWidgetText for wgpu::StorageTextureAccess {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::StorageTextureAccess::WriteOnly => "Write-Only",
            wgpu::StorageTextureAccess::ReadOnly => "Read-Only",
            wgpu::StorageTextureAccess::ReadWrite => "Read-Write",
            wgpu::StorageTextureAccess::Atomic => "Atomic",
        };
        r.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, strum::EnumIter, strum::Display)]
enum ResourceKind {
    Uniform,
    #[strum(to_string = "Texture View")]
    TextureView,
    #[strum(to_string = "Storage Texture")]
    StorageTexture,
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
            ResourceKind::StorageTexture => BindGroupResource::StorageTexture {
                texture_view_id: None,
                access: wgpu::StorageTextureAccess::WriteOnly,
                view_dimension: wgpu::TextureViewDimension::D2,
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
            BindGroupResource::StorageTexture { .. } => ResourceKind::StorageTexture,
            BindGroupResource::Sampler { .. } => ResourceKind::Sampler,
        }
    }
}
