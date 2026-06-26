use std::{hash::Hash, ops::RangeInclusive};

use egui::{Align2, ComboBox, Response, RichText, Ui, Widget, WidgetText};

use crate::{
    project::{ProjectResource, paths::FilePath, storage::Storage},
    ui::components::{
        field::{centered, error_label, row_doc},
        field_docs::{self, FieldDoc},
        resource_icons,
    },
};

/// Trait for types that can be rendered as the label of a combo box entry.
pub trait AsRichText {
    fn as_rich_text(&self) -> RichText;
}

pub fn centered_error(ui: &mut Ui, text: impl Into<RichText>) -> Response {
    let text = text.into();
    centered(ui, |ui| error_label(ui, text)).inner
}

pub fn centered_block(ui: &mut Ui, mut add_contents: impl FnMut(&mut Ui)) {
    let mut block = |ui: &mut Ui| {
        ui.vertical_centered(&mut add_contents);
    };
    let outer = ui.available_rect_before_wrap();
    let size = ui
        .scope_builder(egui::UiBuilder::new().sizing_pass().invisible(), &mut block)
        .response
        .rect
        .size();
    let rect = Align2::CENTER_CENTER.align_size_within_rect(size, outer);
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), block);
}

pub fn section<R>(ui: &mut Ui, title: &str, content: impl FnOnce(&mut Ui) -> R) -> R {
    section_with(ui, title, |_| {}, content)
}

/// Like [`section`], but `header_extra` is rendered inline after the title (for
/// example a help icon).
pub fn section_with<R>(
    ui: &mut Ui,
    title: &str,
    header_extra: impl FnOnce(&mut Ui),
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            top: 8,
            left: 10,
            bottom: 2,
            right: 0,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;
                ui.add(egui::Label::new(
                    egui::RichText::new(title.to_uppercase())
                        .size(12.0)
                        .variation("wght", 600.0),
                ));
                ui.add_space(3.0);
                header_extra(ui);
            });
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

/// Like [`section`], but the heading carries a help icon with the section's
/// documentation.
pub fn section_doc<R>(
    ui: &mut Ui,
    title: &str,
    doc: impl FieldDoc,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    section_with(
        ui,
        title,
        |ui| {
            field_docs::help_icon(ui, doc);
        },
        content,
    )
}

/// Like [`section_doc`], but the tooltip is wider to fit code blocks.
pub fn section_doc_wide<R>(
    ui: &mut Ui,
    title: &str,
    doc: impl FieldDoc,
    content: impl FnOnce(&mut Ui) -> R,
) -> R {
    section_with(
        ui,
        title,
        |ui| {
            field_docs::help_icon_wide(ui, doc);
        },
        content,
    )
}

/// A documented combo-box row: [`value_combo`] with a documented label.
pub fn combo_row_doc<T>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    doc: impl FieldDoc,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    current_value: &mut T,
) -> bool
where
    T: AsRichText + Clone + PartialEq,
{
    row_doc(ui, label, doc, |ui| {
        value_combo(ui, id_salt, options, current_value)
    })
}

/// A documented `f32` drag row: a labelled [`egui::DragValue`] with a documented
/// label.
pub fn f32_drag_row_doc(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    doc: impl FieldDoc,
    value: &mut f32,
    range: RangeInclusive<f32>,
    speed: f64,
    max_decimals: usize,
) -> bool {
    row_doc(ui, label, doc, |ui| {
        egui::DragValue::new(value)
            .speed(speed)
            .max_decimals(max_decimals)
            .range(range)
            .ui(ui)
            .changed()
    })
}

/// A documented `u32` drag row: a labelled [`egui::DragValue`] with a documented
/// label.
pub fn u32_drag_row_doc(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    doc: impl FieldDoc,
    value: &mut u32,
    range: RangeInclusive<u32>,
) -> bool {
    row_doc(ui, label, doc, |ui| {
        egui::DragValue::new(value)
            .speed(1)
            .range(range)
            .ui(ui)
            .changed()
    })
}

/// A documented checkbox row: a labelled checkbox with a documented label.
pub fn checkbox_row_doc(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    doc: impl FieldDoc,
    value: &mut bool,
) -> bool {
    row_doc(ui, label, doc, |ui| ui.checkbox(value, ()).changed())
}

pub fn value_combo<T>(
    ui: &mut Ui,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    current_value: &mut T,
) -> bool
where
    T: AsRichText + Clone + PartialEq,
{
    value_combo_with(ui, id_salt, options, T::as_rich_text, current_value)
}

pub fn value_combo_with<T>(
    ui: &mut Ui,
    id_salt: impl Hash,
    options: impl IntoIterator<Item = T>,
    label_fn: impl Fn(&T) -> RichText,
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

const SELECT_PLACEHOLDER: &str = "Select...";

fn storage_entry_text<R>(ui: &Ui, id: R::Id, label: &str) -> WidgetText
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    let icon = resource_icons::resource_id_icon(id.into());
    resource_icons::icon_text(ui, icon, label)
}

fn storage_label<R>(
    ui: &Ui,
    storage: &Storage<R>,
    id: Option<R::Id>,
    placeholder: WidgetText,
) -> WidgetText
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    match id {
        Some(id) => match storage.get_label(id) {
            Ok(label) => storage_entry_text::<R>(ui, id, label),
            Err(_) => resource_icons::warning_text(ui, "Unknown"),
        },
        None => placeholder,
    }
}

fn select_placeholder(ui: &Ui) -> WidgetText {
    resource_icons::warning_text(ui, SELECT_PLACEHOLDER)
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
        .selected_text(storage_label(ui, storage, *current_value, "None".into()))
        .show_ui(ui, |ui| {
            ui.selectable_value(current_value, None, "None");
            for (id, item) in storage.list_sorted() {
                let text = storage_entry_text::<R>(ui, id, item.label());
                ui.selectable_value(current_value, Some(id), text);
            }
        });
    *current_value != before
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
        .selected_text(storage_label(
            ui,
            storage,
            *current_value,
            select_placeholder(ui),
        ))
        .show_ui(ui, |ui| {
            for (id, item) in storage.list_sorted() {
                let text = storage_entry_text::<R>(ui, id, item.label());
                ui.selectable_value(current_value, Some(id), text);
            }
        });
    *current_value != before
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
            ui,
            storage,
            Some(*current_value),
            select_placeholder(ui),
        ))
        .show_ui(ui, |ui| {
            for (id, item) in storage.list_sorted() {
                let text = storage_entry_text::<R>(ui, id, item.label());
                ui.selectable_value(current_value, id, text);
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
    ui.menu_button(resource_icons::add_text(ui, button_label), |ui| {
        let mut any = false;
        for (id, item) in storage.list_sorted() {
            any = true;
            let text = storage_entry_text::<R>(ui, id, item.label());
            if ui.button(text).clicked() {
                on_pick(id);
                ui.close();
            }
        }
        if !any {
            ui.label(empty_label);
        }
    });
}

pub fn file_combo(
    ui: &mut Ui,
    id_salt: impl Hash,
    files: &[FilePath],
    current_value: &mut Option<FilePath>,
    file_filter: impl Fn(&FilePath) -> bool,
) -> bool {
    let before = current_value.clone();

    let file_text = |ui: &Ui, path: &FilePath| {
        resource_icons::icon_text(ui, resource_icons::file_icon(path), &path.to_string())
    };

    let selected_text = match current_value.as_ref() {
        Some(path) if files.iter().any(|file| file == path) => file_text(ui, path),
        Some(path) => resource_icons::warning_text(ui, &path.to_string()),
        None => select_placeholder(ui),
    };

    ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for file in files.iter().filter(|file| file_filter(file)) {
                let text = file_text(ui, file);
                ui.selectable_value(current_value, Some(file.clone()), text);
            }
        });

    *current_value != before
}
