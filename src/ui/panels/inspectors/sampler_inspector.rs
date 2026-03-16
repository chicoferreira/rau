use egui::{Grid, Widget};

use crate::{
    project::SamplerId,
    ui::{components::selector::selectable_value, pane::StateSnapshot},
};

const ADDRESS_MODES: [wgpu::AddressMode; 4] = [
    wgpu::AddressMode::ClampToEdge,
    wgpu::AddressMode::Repeat,
    wgpu::AddressMode::MirrorRepeat,
    wgpu::AddressMode::ClampToBorder,
];

fn address_mode_label(mode: wgpu::AddressMode) -> &'static str {
    match mode {
        wgpu::AddressMode::ClampToEdge => "Clamp To Edge",
        wgpu::AddressMode::Repeat => "Repeat",
        wgpu::AddressMode::MirrorRepeat => "Mirror Repeat",
        wgpu::AddressMode::ClampToBorder => "Clamp To Border",
    }
}

const FILTER_MODES: [wgpu::FilterMode; 2] = [wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear];

fn filter_mode_label(mode: wgpu::FilterMode) -> &'static str {
    match mode {
        wgpu::FilterMode::Nearest => "Nearest",
        wgpu::FilterMode::Linear => "Linear",
    }
}

const MIPMAP_FILTER_MODES: [wgpu::MipmapFilterMode; 2] = [
    wgpu::MipmapFilterMode::Nearest,
    wgpu::MipmapFilterMode::Linear,
];

fn mipmap_filter_mode_label(mode: wgpu::MipmapFilterMode) -> &'static str {
    match mode {
        wgpu::MipmapFilterMode::Nearest => "Nearest",
        wgpu::MipmapFilterMode::Linear => "Linear",
    }
}

const COMPARE_FUNCTIONS: [Option<wgpu::CompareFunction>; 9] = [
    None,
    Some(wgpu::CompareFunction::Never),
    Some(wgpu::CompareFunction::Less),
    Some(wgpu::CompareFunction::Equal),
    Some(wgpu::CompareFunction::LessEqual),
    Some(wgpu::CompareFunction::Greater),
    Some(wgpu::CompareFunction::NotEqual),
    Some(wgpu::CompareFunction::GreaterEqual),
    Some(wgpu::CompareFunction::Always),
];

fn compare_function_label(compare: Option<wgpu::CompareFunction>) -> &'static str {
    match compare {
        None => "None",
        Some(wgpu::CompareFunction::Never) => "Never",
        Some(wgpu::CompareFunction::Less) => "Less",
        Some(wgpu::CompareFunction::Equal) => "Equal",
        Some(wgpu::CompareFunction::LessEqual) => "Less Equal",
        Some(wgpu::CompareFunction::Greater) => "Greater",
        Some(wgpu::CompareFunction::NotEqual) => "Not Equal",
        Some(wgpu::CompareFunction::GreaterEqual) => "Greater Equal",
        Some(wgpu::CompareFunction::Always) => "Always",
    }
}

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
                selectable_value(
                    ui,
                    "address_mode",
                    &mut spec.address_mode,
                    address_mode_label,
                    ADDRESS_MODES,
                );
                ui.end_row();

                ui.label("Mag Filter");
                selectable_value(
                    ui,
                    "mag_filter",
                    &mut spec.mag_filter,
                    filter_mode_label,
                    FILTER_MODES,
                );
                ui.end_row();

                ui.label("Min Filter");
                selectable_value(
                    ui,
                    "min_filter",
                    &mut spec.min_filter,
                    filter_mode_label,
                    FILTER_MODES,
                );
                ui.end_row();

                ui.label("Mipmap Filter");
                selectable_value(
                    ui,
                    "mipmap_filter",
                    &mut spec.mipmap_filter,
                    mipmap_filter_mode_label,
                    MIPMAP_FILTER_MODES,
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
                selectable_value(
                    ui,
                    "compare",
                    &mut spec.compare,
                    compare_function_label,
                    COMPARE_FUNCTIONS,
                );
                ui.end_row();
            });

        if before != spec {
            sampler.set_spec(spec);
        }
    }
}
