use egui::TextEdit;
use egui_extras::syntax_highlighting::SyntectSettings;
use std::sync::OnceLock;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxDefinition, SyntaxSetBuilder};

use crate::{project::ShaderId, ui::pane::StateSnapshot};

const WGSL_SYNTAX: &str = include_str!("WGSL.sublime-syntax");

static WGSL_SYNTAX_SETTINGS: OnceLock<SyntectSettings> = OnceLock::new();

// TODO: move to state to reuse in other places
fn wgsl_settings() -> &'static SyntectSettings {
    WGSL_SYNTAX_SETTINGS.get_or_init(|| {
        let mut builder = SyntaxSetBuilder::new();
        let syntax = SyntaxDefinition::load_from_str(WGSL_SYNTAX, true, Some("wgsl"))
            .expect("WGSL sublime-syntax definition should be valid YAML");
        builder.add(syntax);
        SyntectSettings {
            ps: builder.build(),
            ts: ThemeSet::load_defaults(),
        }
    })
}

impl StateSnapshot<'_> {
    pub fn shader_inspector_ui(&mut self, ui: &mut egui::Ui, shader_id: ShaderId) {
        ui.take_available_space();
        let shader = self.project.shaders.get_mut(shader_id).unwrap();

        let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());

        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
            let mut layout_job = egui_extras::syntax_highlighting::highlight_with(
                ui.ctx(),
                ui.style(),
                &theme,
                text.as_str(),
                "wgsl",
                wgsl_settings(),
            );

            layout_job.wrap.max_rows = usize::MAX;
            ui.fonts_mut(|f| f.layout_job(layout_job))
        };

        TextEdit::multiline(&mut shader.source)
            .font(egui::TextStyle::Monospace)
            .code_editor()
            .desired_rows(10)
            .lock_focus(true)
            .desired_width(f32::INFINITY)
            .layouter(&mut layouter)
            .show(ui);
    }
}
