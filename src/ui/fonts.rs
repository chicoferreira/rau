use std::sync::Arc;

use crate::ui::theme::{font_family, font_weight};

const GEIST_FONT: &[u8] = include_bytes!("fonts/Geist[wght].ttf");
const GEIST_MONO_FONT: &[u8] = include_bytes!("fonts/GeistMono[wght].ttf");

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    add_font_definition(
        &mut fonts,
        "Geist",
        GEIST_FONT,
        egui::FontFamily::Proportional,
    );
    add_font_definition(
        &mut fonts,
        "Geist Mono",
        GEIST_MONO_FONT,
        egui::FontFamily::Monospace,
    );
    add_weighted_font_definition(
        &mut fonts,
        font_family::PROPORTIONAL_BOLD,
        GEIST_FONT,
        font_weight::BOLD,
    );

    add_weighted_font_definition(
        &mut fonts,
        font_family::PROPORTIONAL_SEMI_BOLD,
        GEIST_FONT,
        font_weight::SEMI_BOLD,
    );

    ctx.set_fonts(fonts);
}

fn add_font_definition(
    fonts: &mut egui::FontDefinitions,
    name: impl Into<String>,
    font: &'static [u8],
    font_family: egui::FontFamily,
) {
    add_font_definition_font_data(fonts, name, egui::FontData::from_static(font), font_family);
}

fn add_weighted_font_definition(
    fonts: &mut egui::FontDefinitions,
    name: &'static str,
    font: &'static [u8],
    font_weight: f32,
) {
    let font_tweak = egui::FontTweak {
        coords: egui::epaint::text::VariationCoords::new([(b"wght", font_weight)]),
        ..Default::default()
    };

    let font_data = egui::FontData::from_static(font).tweak(font_tweak);
    let font_family = egui::FontFamily::Name(name.into());
    add_font_definition_font_data(fonts, name, font_data, font_family);
}

fn add_font_definition_font_data(
    fonts: &mut egui::FontDefinitions,
    name: impl Into<String>,
    font_data: egui::FontData,
    font_family: egui::FontFamily,
) {
    let name = name.into();

    let font_data = Arc::new(font_data);
    fonts.font_data.insert(name.clone(), font_data);

    let font_family_entry = fonts.families.entry(font_family);
    font_family_entry.or_default().insert(0, name);
}
