use egui::WidgetText;
use slotmap::Key;

use crate::project::storage::Storage;

pub fn selectable_value<'a, Id: Key + 'a, V: 'a, W: Into<WidgetText> + 'a>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current_value: &mut Option<Id>,
    format_value: impl Fn(Id, &'a V) -> W,
    options: &'a Storage<Id, V>,
) {
    let selected_text = current_value
        .and_then(|id| options.get(id).map(|value| format_value(id, value).into()))
        .unwrap_or("Empty".into());

    egui::ComboBox::new(id_salt, String::new())
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (id, value) in options.list() {
                ui.selectable_value(current_value, Some(id), format_value(id, value));
            }
        });

    // TODO: add button to open the selected item in a new tab
}
