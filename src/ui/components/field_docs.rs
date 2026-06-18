use std::sync::{Arc, Mutex};

use egui::{Response, RichText, Sense, Ui};
pub use egui_commonmark::CommonMarkCache;
use egui_phosphor::regular;

#[derive(Clone, Default)]
struct DocCache(Arc<Mutex<CommonMarkCache>>);

/// Returns the markdown cache stored in egui's memory, creating it on first use.
fn doc_cache(ctx: &egui::Context) -> Arc<Mutex<CommonMarkCache>> {
    let id = egui::Id::new("inspector_field_doc_cache");
    ctx.data_mut(|data| data.get_temp_mut_or_default::<DocCache>(id).0.clone())
}

/// Builds a documentation closure ([`FieldDoc`]) from a markdown string literal.
///
/// The markdown is parsed at compile time (via `egui_commonmark`'s `macros`
/// feature). Pass the result as the `doc` argument of the inspector's `*_doc`
/// helpers:
///
/// ```ignore
/// inspector::combo_row_doc(
///     ui,
///     "Address Mode",
///     field_doc!("How coordinates **outside** `[0, 1]` are sampled."),
///     "address_mode",
///     ADDRESS_MODES,
///     &mut spec.address_mode,
/// );
/// ```
macro_rules! field_doc {
    ($md:literal) => {
        |ui: &mut egui::Ui, cache: &mut $crate::ui::components::field_docs::CommonMarkCache| {
            egui_commonmark::commonmark!(ui, &mut *cache, $md);
        }
    };
}
pub(crate) use field_doc;

pub trait FieldDoc: FnOnce(&mut Ui, &mut CommonMarkCache) {}
impl<F: FnOnce(&mut Ui, &mut CommonMarkCache)> FieldDoc for F {}

/// Default tooltip width for prose documentation.
const DOC_WIDTH: f32 = 300.0;
/// Wider tooltip for wider documentation containing code blocks for example, so
/// lines wrap less.
const DOC_WIDTH_WIDE: f32 = 500.0;

pub fn help_icon(ui: &mut Ui, doc: impl FieldDoc) -> Response {
    help_icon_sized(ui, DOC_WIDTH, doc)
}

pub fn help_icon_wide(ui: &mut Ui, doc: impl FieldDoc) -> Response {
    help_icon_sized(ui, DOC_WIDTH_WIDE, doc)
}

fn help_icon_sized(ui: &mut Ui, max_width: f32, doc: impl FieldDoc) -> Response {
    let icon = RichText::new(regular::INFO).color(ui.visuals().weak_text_color());
    ui.add(egui::Label::new(icon).sense(Sense::hover()))
        .on_hover_ui(|ui| {
            ui.set_max_width(max_width);
            let cache = doc_cache(ui.ctx());
            doc(ui, &mut cache.lock().unwrap());
        })
}
