use std::{hash::Hash, ops::RangeInclusive};

use egui::{ComboBox, Grid, Ui, Widget, WidgetText};

use crate::project::{ProjectResource, paths::FilePath, storage::Storage};

/// Trait for types that can be rendered as the label of a combo box entry.
pub trait AsWidgetText {
    fn as_widget_text(&self) -> WidgetText;
}

pub fn section<R>(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            top: 8,
            left: 10,
            bottom: 2,
            right: 0,
        })
        .show(ui, |ui| {
            ui.add(egui::Label::new(
                egui::RichText::new(title.to_uppercase())
                    .size(12.0)
                    .variation("wght", 600.0),
            ));
        });

    egui::Frame::new()
        .inner_margin(egui::Margin {
            top: 0,
            left: 6,
            bottom: 0,
            right: 0,
        })
        .show(ui, |ui| ui.indent(title, content).inner)
        .inner
}

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

pub fn value_combo<T>(
    ui: &mut Ui,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    current_value: &mut T,
) -> bool
where
    T: AsWidgetText + Clone + PartialEq,
{
    value_combo_with(ui, id_salt, options, T::as_widget_text, current_value)
}

pub fn value_combo_with<T>(
    ui: &mut Ui,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    label_fn: impl Fn(&T) -> WidgetText,
    current_value: &mut T,
) -> bool
where
    T: Clone + PartialEq,
{
    let before = current_value.clone();
    ComboBox::from_id_salt(id_salt)
        .selected_text(label_fn(current_value))
        .show_ui(ui, |ui| {
            for item in options {
                let label = label_fn(&item);
                ui.selectable_value(current_value, item, label);
            }
        });
    *current_value != before
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
    row(ui, label, |ui| {
        value_combo(ui, id_salt, options, current_value)
    })
}

const SELECT_PLACEHOLDER: &str = "Select...";

fn storage_label<R>(storage: &Storage<R>, id: Option<R::Id>, none_label: &str) -> WidgetText
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    match id {
        Some(id) => match storage.get_label(id) {
            Ok(label) => label.into(),
            Err(_) => format!("Unknown {id:?}").into(),
        },
        None => none_label.into(),
    }
}

pub fn storage_opt_combo<R>(
    ui: &mut Ui,
    id_salt: impl Hash,
    storage: &Storage<R>,
    current_value: &mut Option<R::Id>,
) -> bool
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    let before = *current_value;
    ComboBox::from_id_salt(id_salt)
        .selected_text(storage_label(storage, *current_value, "None"))
        .show_ui(ui, |ui| {
            ui.selectable_value(current_value, None, "None");
            for (id, item) in storage.list_sorted() {
                ui.selectable_value(current_value, Some(id), item.label());
            }
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
    row(ui, label, |ui| {
        storage_opt_combo(ui, id_salt, storage, current_value)
    })
}

pub fn storage_combo<R>(
    ui: &mut Ui,
    id_salt: impl Hash,
    storage: &Storage<R>,
    current_value: &mut Option<R::Id>,
) -> bool
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    let before = *current_value;
    ComboBox::from_id_salt(id_salt)
        .selected_text(storage_label(storage, *current_value, SELECT_PLACEHOLDER))
        .show_ui(ui, |ui| {
            for (id, item) in storage.list_sorted() {
                ui.selectable_value(current_value, Some(id), item.label());
            }
        });
    *current_value != before
}

pub fn storage_combo_row<R>(
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
    row(ui, label, |ui| {
        storage_combo(ui, id_salt, storage, current_value)
    })
}

pub fn storage_id_combo<R>(
    ui: &mut Ui,
    id_salt: impl Hash,
    storage: &Storage<R>,
    current_value: &mut R::Id,
) -> bool
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    let before = *current_value;
    ComboBox::from_id_salt(id_salt)
        .selected_text(storage_label(
            storage,
            Some(*current_value),
            SELECT_PLACEHOLDER,
        ))
        .show_ui(ui, |ui| {
            for (id, item) in storage.list_sorted() {
                ui.selectable_value(current_value, id, item.label());
            }
        });
    *current_value != before
}

pub fn add_from_storage_menu<R>(
    ui: &mut Ui,
    button_label: &str,
    storage: &Storage<R>,
    empty_label: &str,
    mut on_pick: impl FnMut(R::Id),
) where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    ui.menu_button(button_label, |ui| {
        let mut any = false;
        for (id, item) in storage.list_sorted() {
            any = true;
            if ui.button(item.label()).clicked() {
                on_pick(id);
                ui.close();
            }
        }
        if !any {
            ui.label(empty_label);
        }
    });
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
