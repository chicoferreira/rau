use egui::{Color32, Response, RichText, Stroke, Ui, Vec2, WidgetText};

use crate::ui::components::resource_icons;

const ACTION_BUTTON_MIN_SIZE: Vec2 = egui::vec2(168.0, 34.0);

pub fn action_button(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    ui.add(egui::Button::new(text).min_size(ACTION_BUTTON_MIN_SIZE))
}

pub fn primary_action_button(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    primary_action_button_sized(ui, text, ACTION_BUTTON_MIN_SIZE)
}

pub fn primary_action_button_sized(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    size: Vec2,
) -> Response {
    filled_action_button(ui, text, size, ui.visuals().selection.bg_fill)
}

pub fn danger_action_button_sized(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    size: Vec2,
) -> Response {
    filled_action_button(ui, text, size, ui.visuals().error_fg_color)
}

pub fn action_button_sized(ui: &mut Ui, text: impl Into<WidgetText>, size: Vec2) -> Response {
    ui.add(egui::Button::new(text).min_size(size))
}

fn filled_action_button(
    ui: &mut Ui,
    text: impl Into<WidgetText>,
    size: Vec2,
    fill: Color32,
) -> Response {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        widgets.inactive.weak_bg_fill = fill;
        widgets.hovered.weak_bg_fill = fill.gamma_multiply(1.3);
        widgets.active.weak_bg_fill = fill.gamma_multiply(1.5);

        ui.add(egui::Button::new(text).min_size(size).stroke(Stroke::NONE))
    })
    .inner
}

pub fn section_header(ui: &mut Ui, icon: resource_icons::Icon, title: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(RichText::new(icon.glyph).size(16.0).color(icon.color));
        ui.label(
            RichText::new(title)
                .size(16.0)
                .variation("wght", 500.0)
                .strong(),
        );
    });

    ui.add_space(10.0);
}

pub fn modal_title(ui: &mut Ui, title: &str, subtitle: &str) {
    ui.label(
        RichText::new(title)
            .size(20.0)
            .variation("wght", 600.0)
            .strong(),
    );

    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).weak());
    }
}

pub fn modal_section_header(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .size(16.0)
            .variation("wght", 500.0)
            .strong(),
    );

    ui.add_space(4.0);
}

pub fn card<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(8)
        .inner_margin(egui::Margin::same(12))
        .show(ui, add_contents)
        .inner
}
