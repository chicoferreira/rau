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
    ui::{
        components::{
            flags_selector::flags_selector,
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

        let entries = bind_group.entries().to_vec();
        if entries.is_empty() {
            ui.label("No entries in bind group");
        }

        let mut edits = Vec::new();
        let mut ctx = BindGroupUiContext {
            edits: &mut edits,
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
                                ui_entry_title(ui, &mut ctx, index);
                            });
                            ui_entry_fields(ui, &mut ctx, index, field);
                        })
                    });
                });
            }
        });

        if let Some(update) = response.final_update() {
            edits.push(BindGroupEdit::Reorder(update));
        }

        ui.add_space(6.0);

        ui.menu_button("Add Entry", |ui| {
            for kind in ResourceKind::iter() {
                if ui.button(kind.to_string()).clicked() {
                    ui.close();
                    let resource = kind.default_value();
                    edits.push(BindGroupEdit::AddEntry(resource));
                }
            }
        });

        apply_bind_group_edits(self, bind_group_id, edits);

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
    edits: &'a mut Vec<BindGroupEdit>,
    uniforms: &'a Storage<Uniform>,
    texture_views: &'a Storage<TextureView>,
    samplers: &'a Storage<Sampler>,
}

fn ui_entry_title(ui: &mut egui::Ui, ctx: &mut BindGroupUiContext, index: usize) {
    ui.horizontal(|ui| {
        ui.add(
            egui::Label::new(format!("Binding {index}"))
                .selectable(false)
                .sense(egui::Sense::click()),
        )
        .context_menu(|ui| {
            if ui.button("Delete Entry").clicked() {
                ctx.edits.push(BindGroupEdit::DeleteEntry(index));
                ui.close();
            }
        });
    });
}

fn ui_entry_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
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

                    let mut visibility = entry.visibility;
                    ui.label("Visibility");
                    const SHADER_STAGE_OPTIONS: &[(wgpu::ShaderStages, &str)] = &[
                        (wgpu::ShaderStages::VERTEX, "COPY_SRC"),
                        (wgpu::ShaderStages::FRAGMENT, "COPY_DST"),
                        (wgpu::ShaderStages::COMPUTE, "TEXTURE_BINDING"),
                    ];
                    flags_selector(ui, "visibility", &mut visibility, SHADER_STAGE_OPTIONS);
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
                        BindGroupResource::StorageTexture {
                            texture_view_id,
                            access,
                            view_dimension,
                        } => ui_storage_texture_fields(
                            ui,
                            ctx,
                            texture_view_id,
                            access,
                            view_dimension,
                        ),
                    };

                    let resource = (current_kind != kind_before)
                        .then_some(current_kind.default_value())
                        .or(resource_from_fields);

                    let updated_entry = BindGroupEntry {
                        resource: resource.unwrap_or(entry.resource),
                        visibility,
                        ..*entry
                    };

                    if updated_entry != *entry {
                        ctx.edits
                            .push(BindGroupEdit::UpdateEntry(index, updated_entry));
                    }
                });
        });
    });
}

enum BindGroupEdit {
    AddEntry(BindGroupResource),
    DeleteEntry(usize),
    UpdateEntry(usize, BindGroupEntry),
    Reorder(egui_dnd::DragUpdate),
}

fn apply_bind_group_edits(
    state: &mut StateSnapshot<'_>,
    bind_group_id: BindGroupId,
    edits: Vec<BindGroupEdit>,
) {
    if edits.is_empty() {
        return;
    }

    if let Ok(bind_group) = state.project.bind_groups.get_mut(bind_group_id) {
        for edit in edits {
            match edit {
                BindGroupEdit::AddEntry(resource) => {
                    bind_group.add_entry(BindGroupEntry::new_vertex_fragment(resource));
                }
                BindGroupEdit::DeleteEntry(index) => bind_group.remove_entry(index),
                BindGroupEdit::UpdateEntry(index, entry) => bind_group.update_entry(index, entry),
                BindGroupEdit::Reorder(update) => {
                    bind_group.reorder_entries(update.from, update.to);
                }
            }
        }
    }
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
        .show_ui_storage_opt_with_none(ui, ctx.uniforms, &mut uniform_id);
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
        .show_ui_storage_opt_with_none(ui, ctx.texture_views, &mut texture_view_id);

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
        .show_ui_storage_opt_with_none(ui, ctx.samplers, &mut sampler_id);
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

fn ui_storage_texture_fields(
    ui: &mut egui::Ui,
    ctx: &mut BindGroupUiContext,
    mut texture_view_id: Option<TextureViewId>,
    mut access: wgpu::StorageTextureAccess,
    mut view_dimension: wgpu::TextureViewDimension,
) -> Option<BindGroupResource> {
    let before = (texture_view_id, access, view_dimension);

    ui.label("Texture View");
    egui::ComboBox::from_id_salt("storage_texture_view")
        .selected_text_storage_opt(ctx.texture_views, texture_view_id)
        .show_ui_storage_opt_with_none(ui, ctx.texture_views, &mut texture_view_id);
    ui.end_row();

    ui.label("Access");
    egui::ComboBox::from_id_salt("storage_texture_access")
        .selected_text(access.as_widget_text())
        .show_ui_list(ui, STORAGE_TEXTURE_ACCESS, &mut access);
    ui.end_row();

    ui.label("Dimension");
    egui::ComboBox::from_id_salt("storage_texture_view_dimension")
        .selected_text(view_dimension.as_widget_text())
        .show_ui_list(ui, TEXTURE_VIEW_DIMENSIONS, &mut view_dimension);
    ui.end_row();

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
