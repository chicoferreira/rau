use crate::{
    project::SamplerId,
    ui::{
        components::{inspector, selector::AsWidgetText},
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

        inspector::field_grid(ui, "sampler_inspector_grid", |ui| {
            inspector::combo_row(
                ui,
                "Address Mode",
                "address_mode",
                ADDRESS_MODES,
                &mut spec.address_mode,
            );
            inspector::combo_row(
                ui,
                "Mag Filter",
                "mag_filter",
                FILTER_MODES,
                &mut spec.mag_filter,
            );
            inspector::combo_row(
                ui,
                "Min Filter",
                "min_filter",
                FILTER_MODES,
                &mut spec.min_filter,
            );
            inspector::combo_row(
                ui,
                "Mipmap Filter",
                "mipmap_filter",
                MIPMAP_FILTER_MODES,
                &mut spec.mipmap_filter,
            );
            inspector::f32_drag_row(
                ui,
                "LOD Min Clamp",
                &mut spec.lod_min_clamp,
                0.0_f32..=f32::MAX,
                0.1,
                2,
            );
            inspector::f32_drag_row(
                ui,
                "LOD Max Clamp",
                &mut spec.lod_max_clamp,
                0.0_f32..=f32::MAX,
                0.1,
                2,
            );
            inspector::combo_row(
                ui,
                "Compare",
                "compare",
                COMPARE_FUNCTIONS,
                &mut spec.compare,
            );
        });

        if before != spec {
            sampler.set_spec(spec);
        }
    }
}
