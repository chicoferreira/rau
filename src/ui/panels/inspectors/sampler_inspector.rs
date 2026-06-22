use strum::IntoEnumIterator;

use crate::{
    project::SamplerId,
    ui::{
        components::{
            field,
            field_docs::field_doc,
            inspector::{self, AsWidgetText},
        },
        pane::StateSnapshot,
    },
    utils::wgpu_utils::AddressMode,
};

impl AsWidgetText for AddressMode {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.label().into()
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

        inspector::section(ui, "Settings", |ui| {
            field::field_grid(ui, "sampler_inspector_grid", |ui| {
                inspector::combo_row_doc(
                    ui,
                    "Address Mode",
                    field_doc!(
                        "How texture coordinates **outside** the `[0, 1]` range are resolved.\n\n\
                        - **Clamp To Edge**: repeat the edge texel.\n\
                        - **Repeat**: tile the texture.\n\
                        - **Mirror Repeat**: tile, flipping every other repeat.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpuaddressmode)"
                    ),
                    "address_mode",
                    AddressMode::iter(),
                    &mut spec.address_mode,
                );
                inspector::combo_row_doc(
                    ui,
                    "Mag Filter",
                    field_doc!(
                        "Filtering applied when the texture is **magnified** (one texel \
                        covers more than one pixel).\n\n\
                        - **Nearest**: pick the closest texel (sharp, blocky).\n\
                        - **Linear**: blend neighbouring texels (smooth).\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpufiltermode)"
                    ),
                    "mag_filter",
                    FILTER_MODES,
                    &mut spec.mag_filter,
                );
                inspector::combo_row_doc(
                    ui,
                    "Min Filter",
                    field_doc!(
                        "Filtering applied when the texture is **minified** (one pixel \
                        covers more than one texel).\n\n\
                        - **Nearest**: pick the closest texel.\n\
                        - **Linear**: blend neighbouring texels.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpufiltermode)"
                    ),
                    "min_filter",
                    FILTER_MODES,
                    &mut spec.min_filter,
                );
                inspector::combo_row_doc(
                    ui,
                    "Mipmap Filter",
                    field_doc!(
                        "How the two nearest **mip levels** are combined when sampling.\n\n\
                        - **Nearest**: use the closest mip level.\n\
                        - **Linear**: blend between mip levels (trilinear filtering).\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpumipmapfiltermode)"
                    ),
                    "mipmap_filter",
                    MIPMAP_FILTER_MODES,
                    &mut spec.mipmap_filter,
                );
                inspector::f32_drag_row_doc(
                    ui,
                    "LOD Min Clamp",
                    field_doc!(
                        "Lower bound on the **level of detail** (mip level) the sampler may select.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpusamplerdescriptor-lodminclamp)"
                    ),
                    &mut spec.lod_min_clamp,
                    0.0_f32..=f32::MAX,
                    0.1,
                    2,
                );
                inspector::f32_drag_row_doc(
                    ui,
                    "LOD Max Clamp",
                    field_doc!(
                        "Upper bound on the **level of detail** (mip level) the sampler may select.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpusamplerdescriptor-lodmaxclamp)"
                    ),
                    &mut spec.lod_max_clamp,
                    0.0_f32..=f32::MAX,
                    0.1,
                    2,
                );
                inspector::combo_row_doc(
                    ui,
                    "Compare",
                    field_doc!(
                        "Optional **comparison function** for depth-comparison sampling \
                        (e.g. shadow maps). Sampled values are compared against a reference \
                        instead of being returned directly.\n\n\
                        Leave as **None** for ordinary textures.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gpucomparefunction)"
                    ),
                    "compare",
                    COMPARE_FUNCTIONS,
                    &mut spec.compare,
                );
            });
        });

        if before != spec {
            sampler.set_spec(spec);
        }
    }
}
