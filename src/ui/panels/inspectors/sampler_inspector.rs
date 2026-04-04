use egui::{Grid, Widget};

use crate::{
    project::SamplerId,
    ui::{
        components::selector::{AsWidgetText, ComboBoxExt},
        pane::StateSnapshot,
    },
};

const ADDRESS_MODES: [wgpu::AddressMode; 4] = [
    wgpu::AddressMode::ClampToEdge,
    wgpu::AddressMode::Repeat,
    wgpu::AddressMode::MirrorRepeat,
    wgpu::AddressMode::ClampToBorder,
];

impl AsWidgetText for wgpu::AddressMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::AddressMode::ClampToEdge => "Clamp To Edge",
            wgpu::AddressMode::Repeat => "Repeat",
            wgpu::AddressMode::MirrorRepeat => "Mirror Repeat",
            wgpu::AddressMode::ClampToBorder => "Clamp To Border",
        };
        r.into()
    }
}

const FILTER_MODES: [wgpu::FilterMode; 2] = [wgpu::FilterMode::Nearest, wgpu::FilterMode::Linear];

impl AsWidgetText for wgpu::FilterMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::FilterMode::Nearest => "Nearest",
            wgpu::FilterMode::Linear => "Linear",
        };
        r.into()
    }
}

const MIPMAP_FILTER_MODES: [wgpu::MipmapFilterMode; 2] = [
    wgpu::MipmapFilterMode::Nearest,
    wgpu::MipmapFilterMode::Linear,
];

impl AsWidgetText for wgpu::MipmapFilterMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            wgpu::MipmapFilterMode::Nearest => "Nearest",
            wgpu::MipmapFilterMode::Linear => "Linear",
        };
        r.into()
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

impl AsWidgetText for Option<wgpu::CompareFunction> {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            None => "None",
            Some(wgpu::CompareFunction::Never) => "Never",
            Some(wgpu::CompareFunction::Less) => "Less",
            Some(wgpu::CompareFunction::Equal) => "Equal",
            Some(wgpu::CompareFunction::LessEqual) => "Less Equal",
            Some(wgpu::CompareFunction::Greater) => "Greater",
            Some(wgpu::CompareFunction::NotEqual) => "Not Equal",
            Some(wgpu::CompareFunction::GreaterEqual) => "Greater Equal",
            Some(wgpu::CompareFunction::Always) => "Always",
        };
        r.into()
    }
}

impl StateSnapshot<'_> {
    pub fn sampler_inspector_ui(&mut self, ui: &mut egui::Ui, sampler_id: SamplerId) {
        let Ok(sampler) = self.project.samplers.get_mut(sampler_id) else {
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

                egui::ComboBox::from_id_salt("address_mode")
                    .selected_text(spec.address_mode.as_widget_text())
                    .show_ui_list(ui, ADDRESS_MODES, &mut spec.address_mode);

                ui.end_row();

                ui.label("Mag Filter");
                egui::ComboBox::from_id_salt("mag_filter")
                    .selected_text(spec.mag_filter.as_widget_text())
                    .show_ui_list(ui, FILTER_MODES, &mut spec.mag_filter);
                ui.end_row();

                ui.label("Min Filter");
                egui::ComboBox::from_id_salt("min_filter")
                    .selected_text(spec.min_filter.as_widget_text())
                    .show_ui_list(ui, FILTER_MODES, &mut spec.min_filter);
                ui.end_row();

                ui.label("Mipmap Filter");
                egui::ComboBox::from_id_salt("mipmap_filter")
                    .selected_text(spec.mipmap_filter.as_widget_text())
                    .show_ui_list(ui, MIPMAP_FILTER_MODES, &mut spec.mipmap_filter);
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
                egui::ComboBox::from_id_salt("compare")
                    .selected_text(spec.compare.as_widget_text())
                    .show_ui_list(ui, COMPARE_FUNCTIONS, &mut spec.compare);
                ui.end_row();
            });

        if before != spec {
            sampler.set_spec(spec);
        }
    }
}
