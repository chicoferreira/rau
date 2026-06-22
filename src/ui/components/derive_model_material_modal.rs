use egui::RichText;
use egui_phosphor::regular;
use strum::IntoEnumIterator;

use crate::{
    error::{AppError, AppResult},
    project::{
        ModelId, Project, ProjectResource, RuntimeProject,
        paths::FilePath,
        resource::{
            model::{Material, ModelRuntime, TextureType},
            sampler::Sampler,
        },
        storage::Storage,
    },
    ui::components::{field, inspector, main_menu::menu_widgets, resource_icons},
    utils::{
        derive::default_texture_format,
        derive_modal_material::{MaterialBindGroupsConfig, SamplerSetting},
        texture_format::TextureFormat,
    },
};

/// Fixed width of the modal.
const MODAL_WIDTH: f32 = 460.0;

/// Modal that creates one bind group per material of a model, along with the
/// textures and texture views for the images the materials reference.
///
/// All created bind groups share the same entry layout, since the model
/// requires every material bind group layout to match. Because of that, only
/// texture types present in *every* material can be included.
pub struct MaterialBindGroupsModal {
    model_id: ModelId,
    model_label: String,
    material_count: usize,
    texture_types: Vec<TextureTypeRow>,
    sampler_setting: SamplerSetting,
}

struct TextureTypeRow {
    texture_type: TextureType,
    selected: bool,
    /// Format the created textures of this type will use. Defaults to a value
    /// derived from the texture type but is user-overridable.
    format: TextureFormat,
    /// Number of materials that reference a texture of this type.
    present_count: usize,
    /// Referenced texture files that don't exist in the project.
    missing_files: Vec<FilePath>,
}

/// Formats offered for material textures. Depth formats are intentionally left
/// out since they don't make sense for sampled color/data textures.
const MATERIAL_TEXTURE_FORMATS: [TextureFormat; 4] = TextureFormat::COLOR;

enum MaterialBindGroupsModalResponse {
    Create,
    Cancel,
}

impl MaterialBindGroupsModal {
    pub fn open(
        project: &Project,
        runtime_project: &RuntimeProject,
        files: Option<&[FilePath]>,
        toasts: &mut egui_notify::Toasts,
        model_id: ModelId,
    ) -> Option<Self> {
        let model_label = project
            .models
            .get_label(model_id)
            .unwrap_or("Model")
            .to_string();

        match runtime_project.models.get_init(model_id) {
            Ok(Some(model_runtime)) => {
                if model_runtime.materials().is_empty() {
                    toasts_log_error!(
                        toasts,
                        "Model \"{model_label}\" has no materials to create bind groups from"
                    );
                } else {
                    let materials = model_runtime.materials();
                    return Some(Self::new(model_id, model_label, materials, files));
                }
            }
            Ok(None) => {
                toasts_log_error!(
                    toasts,
                    "Model \"{model_label}\" is still loading, try again in a moment"
                );
            }
            Err(error) => {
                toasts_log_error!(
                    toasts,
                    "Failed to create bind groups from materials: {error}"
                );
            }
        }

        None
    }

    pub fn new(
        model_id: ModelId,
        model_label: String,
        materials: &[Material],
        files: Option<&[FilePath]>,
    ) -> Self {
        let material_count = materials.len();

        let texture_types = TextureType::iter()
            .filter_map(|texture_type| {
                let paths = materials
                    .iter()
                    .filter_map(|material| material.get_texture_path(texture_type))
                    .collect::<Vec<_>>();

                if paths.is_empty() {
                    return None;
                }

                let mut missing_files: Vec<FilePath> = Vec::new();
                if let Some(files) = files {
                    for path in &paths {
                        if !files.contains(path) && !missing_files.contains(path) {
                            missing_files.push((*path).clone());
                        }
                    }
                }

                Some(TextureTypeRow {
                    texture_type,
                    selected: paths.len() == material_count,
                    format: default_texture_format(texture_type),
                    present_count: paths.len(),
                    missing_files,
                })
            })
            .collect();

        Self {
            model_id,
            model_label,
            material_count,
            texture_types,
            sampler_setting: SamplerSetting::CreateNew,
        }
    }

    /// Renders the modal and, on confirmation, creates the resources and assigns
    /// the bind groups to the model. Returns `true` while the modal should remain
    /// open, and `false` once it has been confirmed or dismissed.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        project: &mut Project,
        runtime_project: &RuntimeProject,
        toasts: &mut egui_notify::Toasts,
    ) -> bool {
        let Some(response) = self.render_ui(ui, &project.samplers) else {
            return true;
        };

        match response {
            MaterialBindGroupsModalResponse::Cancel => false,
            MaterialBindGroupsModalResponse::Create => {
                let result = match runtime_project.models.get_init(self.model_id) {
                    Ok(Some(model_runtime)) => self
                        .apply(project, model_runtime)
                        .map(|()| model_runtime.materials().len()),
                    Ok(None) => Err(AppError::uninit_field("Model Runtime")),
                    Err(error) => Err(error),
                };

                match result {
                    Ok(material_count) => {
                        toasts.success(format!(
                            "Created bind groups for {material_count} material(s)"
                        ));
                    }
                    Err(error) => {
                        toasts_log_error!(
                            toasts,
                            "Failed to create bind groups from materials: {error}"
                        );
                    }
                }

                false
            }
        }
    }

    fn render_ui(
        &mut self,
        ui: &mut egui::Ui,
        samplers: &Storage<Sampler>,
    ) -> Option<MaterialBindGroupsModalResponse> {
        let mut result = None;

        let frame = egui::Frame::popup(ui.style()).inner_margin(20);
        let response = egui::Modal::new(egui::Id::new("material_bind_groups_modal"))
            .frame(frame)
            .show(ui.ctx(), |ui| {
                ui.set_width(MODAL_WIDTH);

                menu_widgets::modal_title(
                    ui,
                    "Create Bind Groups from Materials",
                    &format!(
                        "Creates a bind group for each of the {} material(s) of \"{}\", along with the textures and texture views for the images they reference.",
                        self.material_count, self.model_label,
                    ),
                );

                ui.add_space(10.0);
                self.texture_types_ui(ui);

                ui.add_space(10.0);
                self.sampler_ui(ui, samplers);

                ui.add_space(10.0);
                ui.weak("This will replace the bind groups currently assigned to the model materials.");

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                    if menu_widgets::action_button_sized(ui, "Cancel", egui::vec2(half, 34.0))
                        .clicked()
                    {
                        result = Some(MaterialBindGroupsModalResponse::Cancel);
                    }

                    let any_type_selected = self.texture_types.iter().any(|row| row.selected);
                    let size = egui::vec2(ui.available_width(), 34.0);
                    ui.add_enabled_ui(any_type_selected, |ui| {
                        let label = resource_icons::monochrome_icon_text(
                            ui,
                            regular::MAGIC_WAND,
                            egui::Color32::WHITE,
                            "Create",
                        );
                        if menu_widgets::primary_action_button_sized(ui, label, size)
                            .on_disabled_hover_text("Select at least one texture type.")
                            .clicked()
                        {
                            result = Some(MaterialBindGroupsModalResponse::Create);
                        }
                    });
                });
            });

        if result.is_none() && response.should_close() {
            result = Some(MaterialBindGroupsModalResponse::Cancel);
        }

        result
    }

    fn texture_types_ui(&mut self, ui: &mut egui::Ui) {
        if self.texture_types.is_empty() {
            ui.weak("The materials of this model don't reference any texture files.");
            return;
        }

        menu_widgets::modal_section_header(ui, "Texture types to include");

        let material_count = self.material_count;
        field::field_grid(ui, "material_bind_groups_modal_types", |ui| {
            for row in &mut self.texture_types {
                field::row(ui, row.texture_type.to_string(), |ui| {
                    ui.horizontal(|ui| {
                        let enabled = row.present_count == material_count;

                        ui.add_enabled(
                            enabled,
                            egui::Checkbox::new(
                                &mut row.selected,
                                format!("{}/{material_count}", row.present_count),
                            ),
                        )
                        .on_disabled_hover_text(
                            "Not every material has this texture type, so including it \
                             would make the material bind group layouts differ.",
                        );

                        inspector::value_combo(
                            ui,
                            ("material_bind_groups_modal_format", row.texture_type),
                            MATERIAL_TEXTURE_FORMATS,
                            &mut row.format,
                        );

                        if !row.missing_files.is_empty() {
                            let missing = row
                                .missing_files
                                .iter()
                                .map(FilePath::to_string)
                                .collect::<Vec<_>>()
                                .join("\n");
                            ui.label(RichText::new("⚠").color(ui.visuals().warn_fg_color))
                                .on_hover_text(format!("Missing files:\n{missing}"));
                        }
                    });
                });
            }
        });
    }

    fn sampler_ui(&mut self, ui: &mut egui::Ui, samplers: &Storage<Sampler>) {
        menu_widgets::modal_section_header(ui, "Sampler to include");

        let selected_text = match &self.sampler_setting {
            SamplerSetting::None => "None".to_string(),
            SamplerSetting::CreateNew => "Create new sampler".to_string(),
            SamplerSetting::Existing(id) => {
                samplers.get_label(*id).unwrap_or("Unknown").to_string()
            }
        };

        field::field_grid(ui, "material_bind_groups_modal_sampler", |ui| {
            field::row(ui, "Sampler binding", |ui| {
                egui::ComboBox::from_id_salt("material_bind_groups_modal_sampler_combo")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.sampler_setting,
                            SamplerSetting::None,
                            "None",
                        );
                        ui.selectable_value(
                            &mut self.sampler_setting,
                            SamplerSetting::CreateNew,
                            "Create new sampler",
                        );
                        for (id, sampler) in samplers.list_sorted() {
                            ui.selectable_value(
                                &mut self.sampler_setting,
                                SamplerSetting::Existing(id),
                                sampler.label(),
                            );
                        }
                    });
            });
        });
    }

    /// Builds the [`MaterialBindGroupsConfig`] from the modal state, creates the
    /// resources, and assigns the resulting bind groups to the model.
    fn apply(&self, project: &mut Project, model_runtime: &ModelRuntime) -> AppResult<()> {
        let config = MaterialBindGroupsConfig {
            textures: self
                .texture_types
                .iter()
                .filter(|row| row.selected)
                .map(|row| (row.texture_type, row.format))
                .collect(),
            sampler: self.sampler_setting.clone(),
        };

        let bind_group_ids =
            config.create_bind_groups(project, model_runtime.materials(), &self.model_label)?;

        project
            .models
            .get_mut(self.model_id)?
            .set_material_bind_group_ids(bind_group_ids);

        Ok(())
    }
}
