use egui::{Grid, Widget};

use crate::{
    project::SamplerId,
    ui::{components::selector::combo_grid_row, pane::StateSnapshot},
};

const ADDRESS_MODES: &[(wgpu::AddressMode, &str)] = &[
    (wgpu::AddressMode::ClampToEdge, "Clamp To Edge"),
    (wgpu::AddressMode::Repeat, "Repeat"),
    (wgpu::AddressMode::MirrorRepeat, "Mirror Repeat"),
    (wgpu::AddressMode::ClampToBorder, "Clamp To Border"),
];

const FILTER_MODES: &[(wgpu::FilterMode, &str)] = &[
    (wgpu::FilterMode::Nearest, "Nearest"),
    (wgpu::FilterMode::Linear, "Linear"),
];

const MIPMAP_FILTER_MODES: &[(wgpu::MipmapFilterMode, &str)] = &[
    (wgpu::MipmapFilterMode::Nearest, "Nearest"),
    (wgpu::MipmapFilterMode::Linear, "Linear"),
];

const COMPARE_FUNCTIONS: &[(Option<wgpu::CompareFunction>, &str)] = &[
    (None, "None"),
    (Some(wgpu::CompareFunction::Never), "Never"),
    (Some(wgpu::CompareFunction::Less), "Less"),
    (Some(wgpu::CompareFunction::Equal), "Equal"),
    (Some(wgpu::CompareFunction::LessEqual), "Less Equal"),
    (Some(wgpu::CompareFunction::Greater), "Greater"),
    (Some(wgpu::CompareFunction::NotEqual), "Not Equal"),
    (Some(wgpu::CompareFunction::GreaterEqual), "Greater Equal"),
    (Some(wgpu::CompareFunction::Always), "Always"),
];

impl StateSnapshot<'_> {
    pub fn sampler_inspector_ui(&mut self, ui: &mut egui::Ui, sampler_id: SamplerId) {
        let Some(sampler) = self.project.samplers.get_mut(sampler_id) else {
            ui.label("Sampler couldn't be found.");
            return;
        };

        let mut spec = sampler.spec().clone();
        let before = spec.clone();

        Grid::new("sampler_inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Address Mode");
                combo_grid_row(
                    ui,
                    "address_mode",
                    &mut spec.address_mode,
                    ADDRESS_MODES,
                    "",
                );
                ui.end_row();

                ui.label("Mag Filter");
                combo_grid_row(ui, "mag_filter", &mut spec.mag_filter, FILTER_MODES, "");
                ui.end_row();

                ui.label("Min Filter");
                combo_grid_row(ui, "min_filter", &mut spec.min_filter, FILTER_MODES, "");
                ui.end_row();

                ui.label("Mipmap Filter");
                combo_grid_row(
                    ui,
                    "mipmap_filter",
                    &mut spec.mipmap_filter,
                    MIPMAP_FILTER_MODES,
                    "",
                );
                ui.end_row();

                ui.label("LOD Min Clamp");
                egui::DragValue::new(&mut spec.lod_min_clamp)
                    .speed(0.1)
                    .max_decimals(2)
                    .range(0.0_f32..=f32::MAX)
                    .ui(ui);
                ui.end_row();

                ui.label("LOD Max Clamp");
                egui::DragValue::new(&mut spec.lod_max_clamp)
                    .speed(0.1)
                    .max_decimals(2)
                    .range(0.0_f32..=f32::MAX)
                    .ui(ui);
                ui.end_row();

                ui.label("Compare");
                combo_grid_row(ui, "compare", &mut spec.compare, COMPARE_FUNCTIONS, "");
                ui.end_row();
            });

        if before != spec {
            sampler.set_spec(spec);
        }
    }
}
