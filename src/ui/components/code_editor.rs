use std::sync::OnceLock;

use egui_extras::syntax_highlighting::SyntectSettings;
use syntect::{
    highlighting::ThemeSet,
    parsing::{SyntaxDefinition, SyntaxSetBuilder},
};

use crate::project::shader_code::{self, Language, ShaderGenCtx, ShaderInterface};

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
        SyntectSettings {
            ps: builder.build(),
            ts: ThemeSet::load_defaults(),
        }
    })
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
            let backend = Language::Wgsl;
            let code = shader_code::render(item, ctx, backend);
            code_view(ui, &code, backend.highlight_extension());
        });
}

pub fn code_view(ui: &mut egui::Ui, code: &str, extension: &str) {
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
    let layout_job = egui_extras::syntax_highlighting::highlight_with(
        ui.ctx(),
        ui.style(),
        &theme,
        code,
        extension,
        syntax_settings(),
    );

    egui::Frame::group(ui.style())
        .fill(ui.visuals().extreme_bg_color)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(layout_job).selectable(true));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui.small_button("Copy").clicked() {
                        ui.ctx().copy_text(code.to_owned());
                    }
                });
            });
        });
}

pub fn code_editor(ui: &mut egui::Ui, text: &mut String, extension: &str) -> egui::Response {
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());

    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
        let mut layout_job = egui_extras::syntax_highlighting::highlight_with(
            ui.ctx(),
            ui.style(),
            &theme,
            text.as_str(),
            extension,
            syntax_settings(),
        );

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
