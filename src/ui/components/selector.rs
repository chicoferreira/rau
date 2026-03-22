use egui::WidgetText;
use slotmap::Key;

use crate::project::{ProjectResourceId, storage::Storage};

pub fn selectable_value_storage<
    'a,
    Id: Key + 'a + Into<ProjectResourceId>,
    V: 'a,
    W: Into<WidgetText> + 'a,
>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current_value: &mut Option<Id>,
    format_value: impl Fn(Id, &'a V) -> W,
    options: &'a Storage<Id, V>,
) {
    let selected_text: WidgetText = match current_value {
        None => "Empty".into(),
        Some(id) => match options.get(*id) {
            Ok(value) => format_value(*id, value).into(),
            Err(_) => format!("Unknown {id:?}").into(),
        },
    };

    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (id, value) in options.list() {
                ui.selectable_value(current_value, Some(id), format_value(id, value));
            }
        });

    // TODO: add button to open the selected item in a new tab
}

pub fn selectable_value<V: PartialEq + Clone, W: Into<WidgetText>>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    current_value: &mut V,
    format_value: impl Fn(V) -> W,
    options: impl IntoIterator<Item = V>,
) {
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(format_value(current_value.clone()))
        .show_ui(ui, |ui| {
            for value in options {
                ui.selectable_value(current_value, value.clone(), format_value(value.clone()));
            }
        });
}
