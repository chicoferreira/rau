use egui::WidgetText;
use slotmap::Key;

use crate::project::storage::Storage;

// TODO: Merge these functions into one
pub fn selectable_value<'a, Id: Key + 'a, V: 'a, W: Into<WidgetText> + 'a>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current_value: &mut Option<Id>,
    format_value: impl Fn(Id, &'a V) -> W,
    options: &'a Storage<Id, V>,
) {
    let selected_text: WidgetText = match current_value {
        None => "Empty".into(),
        Some(id) => match options.get(*id) {
            Some(value) => format_value(*id, value).into(),
            None => format!("Unknown {id:?}").into(),
        },
    };

    egui::ComboBox::new(id_salt, String::new())
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (id, value) in options.list() {
                ui.selectable_value(current_value, Some(id), format_value(id, value));
            }
        });

    // TODO: add button to open the selected item in a new tab
}

// TODO: Redo this function
pub fn combo_grid_row<T>(
    ui: &mut egui::Ui,
    combo_id: impl std::hash::Hash,
    current: &mut T,
    options: &[(T, &str)],
    empty_msg: &str,
) where
    T: PartialEq + Copy,
{
    let selected_text = options
        .iter()
        .find(|(v, _)| v == current)
        .map(|(_, l)| *l)
        .unwrap_or("Unknown");
    egui::ComboBox::from_id_salt(combo_id)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (value, label) in options {
                ui.selectable_value(current, *value, *label);
            }
            if options.is_empty() {
                ui.label(egui::RichText::new(empty_msg).weak());
            }
        });
}
