use std::sync::Arc;

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

    ctx.set_fonts(fonts);
}

fn add_font_definition(
    fonts: &mut egui::FontDefinitions,
    name: impl Into<String>,
    font: &'static [u8],
    font_family: egui::FontFamily,
) {
    let name = name.into();

    let font_data = Arc::new(egui::FontData::from_static(font));
    fonts.font_data.insert(name.clone(), font_data);

    let font_family_entry = fonts.families.entry(font_family);
    font_family_entry.or_default().insert(0, name);
}
