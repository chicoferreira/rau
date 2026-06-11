use std::sync::OnceLock;

use egui_extras::syntax_highlighting::SyntectSettings;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxDefinition, SyntaxSetBuilder},
};

use crate::utils::shader_preview::{self, Language, ShaderGenCtx, ShaderInterface};

static SYNTAX_SETTINGS: OnceLock<SyntectSettings> = OnceLock::new();

fn syntax_settings() -> &'static SyntectSettings {
    macro_rules! register_syntax {
        ($builder:expr, $source:expr, $name:expr) => {
            let source = include_str!($source);
            let syntax = SyntaxDefinition::load_from_str(source, true, Some($name))
                .expect(&format!("{} syntax definition should be valid YAML", $name));
            $builder.add(syntax);
        };
    }

    SYNTAX_SETTINGS.get_or_init(|| {
        let mut builder = SyntaxSetBuilder::new();
        register_syntax!(builder, "syntaxes/WGSL.sublime-syntax", "wgsl");
        register_syntax!(builder, "syntaxes/WavefrontOBJ.sublime-syntax", "obj");
        register_syntax!(builder, "syntaxes/WavefrontMTL.sublime-syntax", "mtl");
        register_syntax!(builder, "syntaxes/JSON.sublime-syntax", "json");
        register_syntax!(builder, "syntaxes/GLSL.sublime-syntax", "glsl");
        SyntectSettings {
            ps: builder.build(),
            ts: ThemeSet::load_defaults(),
        }
    })
}

fn layout_job(ui: &egui::Ui, code: &str, extension: &str) -> egui::text::LayoutJob {
    use egui_extras::syntax_highlighting::highlight_with;
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
    let (ctx, style) = (ui.ctx(), ui.style());
    let settings = syntax_settings();

    highlight_with(ctx, style, &theme, code, extension, settings)
}

pub fn shader_code_section(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    item: &impl ShaderInterface,
    ctx: &ShaderGenCtx,
) {
    egui::CollapsingHeader::new("Shader code")
        .id_salt(id_salt)
        .default_open(true)
        .show(ui, |ui| {
            let language_id = egui::Id::new("shader_code_language");
            let language = ui
                .ctx()
                .data(|data| data.get_temp(language_id))
                .unwrap_or(Language::Wgsl);

            let code = shader_preview::render(item, ctx, language);
            let layout_job = layout_job(ui, &code, language.highlight_extension());

            egui::Frame::group(ui.style())
                .fill(ui.visuals().extreme_bg_color)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(layout_job).selectable(true));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            if ui.small_button("Copy").clicked() {
                                ui.ctx().copy_text(code.clone());
                            }

                            let next = match language {
                                Language::Wgsl => Language::Glsl,
                                Language::Glsl => Language::Wgsl,
                            };

                            if ui
                                .small_button(language.to_string())
                                .on_hover_text(format!("Switch to {}", next))
                                .clicked()
                            {
                                ui.ctx()
                                    .data_mut(|data| data.insert_temp(language_id, next));
                            }
                        });
                    });
                });
        });
}

pub fn highlighted_label(ui: &mut egui::Ui, code: &str, extension: &str) -> egui::Response {
    let layout_job = layout_job(ui, code, extension);
    ui.add(egui::Label::new(layout_job).selectable(true))
}

pub fn code_editor(ui: &mut egui::Ui, text: &mut String, extension: &str) -> egui::Response {
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
        let mut layout_job = layout_job(ui, text.as_str(), extension);

        layout_job.wrap.max_rows = usize::MAX;
        ui.fonts_mut(|f| f.layout_job(layout_job))
    };

    ui.add(
        egui::TextEdit::multiline(text)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(24)
            .desired_width(f32::INFINITY)
            .lock_focus(true)
            .layouter(&mut layouter),
    )
}
