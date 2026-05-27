use std::{hash::Hash, ops::RangeInclusive};

use egui::{ComboBox, Grid, Ui, Widget, WidgetText};

use crate::{
    project::{ProjectResource, paths::FilePath, storage::Storage},
    ui::components::selector::{AsWidgetText, ComboBoxExt},
};

pub fn field_grid<R>(
    ui: &mut Ui,
    id_salt: impl Hash,
    add_rows: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    Grid::new(id_salt)
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, add_rows)
}

pub fn row<R>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.label(label);
    let result = add_control(ui);
    ui.end_row();
    result
}

pub fn combo_row<T>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    current_value: &mut T,
) -> bool
where
    T: AsWidgetText + Clone + PartialEq,
{
    let before = current_value.clone();
    row(ui, label, |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text(current_value.as_widget_text())
            .show_ui_list(ui, options, current_value);
    });
    *current_value != before
}

pub fn storage_opt_combo_row<R>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    id_salt: impl Hash,
    storage: &Storage<R>,
    current_value: &mut Option<R::Id>,
) -> bool
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    let before = *current_value;
    row(ui, label, |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text_storage_opt(storage, *current_value)
            .show_ui_storage_opt_with_none(ui, storage, current_value);
    });
    *current_value != before
}

pub fn file_opt_combo_row(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    id_salt: impl Hash,
    files: &[FilePath],
    current_value: &mut Option<FilePath>,
    file_filter: impl Fn(&FilePath) -> bool,
) -> bool {
    let before = current_value.clone();

    let display_label = |path: &FilePath| path.to_string();

    let selected_text = current_value
        .as_ref()
        .map_or_else(|| "None".into(), display_label);

    row(ui, label, |ui| {
        ComboBox::from_id_salt(id_salt)
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(current_value, None, "None");
                for file in files.iter().filter(|file| file_filter(file)) {
                    ui.selectable_value(current_value, Some(file.clone()), display_label(file));
                }
            });
    });

    *current_value != before
}

pub fn checkbox_row(ui: &mut Ui, label: impl Into<WidgetText>, value: &mut bool) -> bool {
    row(ui, label, |ui| ui.checkbox(value, ()).changed())
}

pub fn f32_drag_row(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut f32,
    range: RangeInclusive<f32>,
    speed: f64,
    max_decimals: usize,
) -> bool {
    row(ui, label, |ui| {
        egui::DragValue::new(value)
            .speed(speed)
            .max_decimals(max_decimals)
            .range(range)
            .ui(ui)
            .changed()
    })
}

pub fn u32_drag_row(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    value: &mut u32,
    range: RangeInclusive<u32>,
) -> bool {
    row(ui, label, |ui| {
        egui::DragValue::new(value)
            .speed(1)
            .range(range)
            .ui(ui)
            .changed()
    })
}
