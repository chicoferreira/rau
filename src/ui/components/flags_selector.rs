use std::hash::Hash;

use bitflags::Flags;
use egui::{ComboBox, PopupCloseBehavior, Ui};

pub fn flags_selector<F>(
    ui: &mut Ui,
    id_salt: impl Hash,
    value: &mut F,
    options: &[(F, &str)],
) -> bool
where
    F: Flags + PartialEq + Copy,
{
    let before = *value;

    let summary = if value.is_empty() {
        "None".to_string()
    } else {
        options
            .iter()
            .filter(|(flag, _)| value.contains(*flag))
            .map(|(_, name)| *name)
            .collect::<Vec<_>>()
            .join(" | ")
    };

    ComboBox::from_id_salt(id_salt)
        .selected_text(summary)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            for &(flag, name) in options {
                let mut enabled = value.contains(flag);
                if ui.checkbox(&mut enabled, name).changed() {
                    value.set(flag, enabled);
                }
            }
        });

    *value != before
}
