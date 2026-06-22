use std::hash::Hash;

use egui::{Direction, Grid, InnerResponse, Layout, Response, RichText, Ui, WidgetText};

use crate::ui::components::field_docs::{self, FieldDoc};

pub fn centered<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.with_layout(
        Layout::centered_and_justified(Direction::TopDown).with_cross_justify(false),
        add_contents,
    )
}

pub fn error_label(ui: &mut Ui, text: impl Into<RichText>) -> Response {
    let text = text.into().color(ui.visuals().error_fg_color);
    ui.add(egui::Label::new(text).selectable(true))
}

pub fn weak_label(ui: &mut Ui, text: impl Into<RichText>) -> Response {
    ui.label(text.into().size(11.0).color(ui.visuals().weak_text_color()))
}

pub fn spinner(ui: &mut Ui) -> Response {
    ui.add(egui::Spinner::new().size(ui.text_style_height(&egui::TextStyle::Body)))
}

pub fn field_grid<R>(
    ui: &mut Ui,
    id_salt: impl Hash,
    add_rows: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
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

fn doc_label(ui: &mut Ui, label: impl Into<WidgetText>, doc: impl FieldDoc) {
    ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing.x = 0.0;
        ui.label(label);
        ui.add_space(2.0);
        field_docs::help_icon(ui, doc);
    });
}

pub fn row_doc<R>(
    ui: &mut Ui,
    label: impl Into<WidgetText>,
    doc: impl FieldDoc,
    add_control: impl FnOnce(&mut Ui) -> R,
) -> R {
    doc_label(ui, label, doc);
    let result = add_control(ui);
    ui.end_row();
    result
}
