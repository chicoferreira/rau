use egui::{Color32, CornerRadius, Stroke, Style, Theme, Visuals, style::WidgetVisuals};

pub fn install(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Dark, install_dark_theme);
    ctx.style_mut_of(Theme::Light, install_light_theme);
}

fn install_dark_theme(style: &mut Style) {
    let accent = Color32::from_rgb(218, 223, 232);
    let mut v = Visuals::dark();

    v.panel_fill = Color32::from_gray(23);
    v.window_fill = Color32::from_gray(23);
    v.extreme_bg_color = Color32::from_gray(10);
    v.code_bg_color = Color32::from_gray(10);
    v.faint_bg_color = Color32::from_gray(32);

    v.warn_fg_color = Color32::from_rgb(232, 181, 94);
    v.error_fg_color = Color32::from_rgb(240, 90, 90);
    v.hyperlink_color = accent;
    v.weak_text_color = Some(Color32::from_gray(100));

    v.window_stroke = Stroke::new(1.0_f32, Color32::from_gray(46));
    v.window_shadow.color = Color32::from_black_alpha(110);
    v.popup_shadow.color = Color32::from_black_alpha(90);

    v.selection.bg_fill = Color32::from_rgb(45, 104, 211);
    v.selection.stroke = Stroke::new(1.0_f32, accent);

    let text = Color32::from_gray(200);
    let text_strong = Color32::from_gray(250);
    let border = Color32::from_gray(46);
    let border_strong = Color32::from_gray(64);

    let egui::style::Widgets {
        noninteractive,
        inactive,
        hovered,
        active,
        open,
    } = &mut v.widgets;

    paint(noninteractive, Color32::from_gray(23), border, text);
    paint(inactive, Color32::from_gray(38), border, text);
    paint(hovered, Color32::from_gray(48), border_strong, text_strong);
    paint(active, Color32::from_gray(58), accent, text_strong);
    paint(open, Color32::from_gray(48), border_strong, text_strong);

    style.visuals = v;
    install_theme(style);
}

fn install_light_theme(style: &mut Style) {
    let accent = Color32::from_rgb(36, 42, 52);
    let mut v = Visuals::light();

    v.panel_fill = Color32::from_gray(240);
    v.window_fill = Color32::from_gray(240);
    v.extreme_bg_color = Color32::from_gray(255);
    v.code_bg_color = Color32::from_gray(255);
    v.faint_bg_color = Color32::from_gray(232);

    v.warn_fg_color = Color32::from_rgb(160, 104, 12);
    v.error_fg_color = Color32::from_rgb(190, 42, 42);
    v.hyperlink_color = accent;
    v.weak_text_color = Some(Color32::from_gray(120));

    v.window_stroke = Stroke::new(1.0_f32, Color32::from_gray(202));
    v.window_shadow.color = Color32::from_black_alpha(40);
    v.popup_shadow.color = Color32::from_black_alpha(30);

    v.selection.bg_fill = Color32::from_rgb(160, 198, 242);
    v.selection.stroke = Stroke::new(1.0_f32, accent);

    let text = Color32::from_gray(52);
    let text_strong = Color32::from_gray(16);
    let border = Color32::from_gray(202);
    let border_strong = Color32::from_gray(162);

    let egui::style::Widgets {
        noninteractive,
        inactive,
        hovered,
        active,
        open,
    } = &mut v.widgets;

    paint(noninteractive, Color32::from_gray(240), border, text);
    paint(inactive, Color32::from_gray(252), border, text);
    paint(hovered, Color32::from_gray(238), border_strong, text_strong);
    paint(active, Color32::from_gray(224), accent, text_strong);
    paint(open, Color32::from_gray(238), border_strong, text_strong);

    style.visuals = v;
    install_theme(style);
}

fn paint(widget: &mut WidgetVisuals, bg: Color32, border: Color32, text: Color32) {
    widget.bg_fill = bg;
    widget.weak_bg_fill = bg;
    widget.bg_stroke = Stroke::new(1.0_f32, border);
    widget.fg_stroke = Stroke::new(1.0_f32, text);
}

fn install_theme(style: &mut Style) {
    style.interaction.selectable_labels = false;
    style.url_in_tooltip = true;

    let v = &mut style.visuals;
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.slider_trailing_fill = true;
    v.window_corner_radius = CornerRadius::same(7);
    v.menu_corner_radius = CornerRadius::same(5);

    v.window_shadow.offset = [0, 6];
    v.window_shadow.blur = 18;
    v.window_shadow.spread = 0;
    v.popup_shadow.offset = [0, 4];
    v.popup_shadow.blur = 12;
    v.popup_shadow.spread = 0;

    let w = &mut v.widgets;
    for widget in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
    }

    let spacing = &mut style.spacing;
    spacing.button_padding.x = 6.0;
    spacing.menu_margin = egui::Margin::same(6);

    spacing.scroll.bar_width = 9.0;
    spacing.scroll.floating = true;
    spacing.scroll.floating_allocated_width = 0.0;
    spacing.indent = 25.0;
}
