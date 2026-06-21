use egui::{Color32, CornerRadius, Shadow, Stroke};

struct Palette;

#[allow(dead_code)]
impl Palette {
    const BG_DEEP: Color32 = Color32::from_gray(10);
    const BG_BASE: Color32 = Color32::from_gray(23);
    const BG_PANEL: Color32 = Color32::from_gray(23);
    const BG_FAINT: Color32 = Color32::from_gray(32);

    const SURFACE: Color32 = Color32::from_gray(38);
    const SURFACE_HOVER: Color32 = Color32::from_gray(48);
    const SURFACE_ACTIVE: Color32 = Color32::from_gray(58);

    const BORDER: Color32 = Color32::from_gray(46);
    const BORDER_STRONG: Color32 = Color32::from_gray(64);

    const TEXT_STRONG: Color32 = Color32::from_gray(250);
    const TEXT: Color32 = Color32::from_gray(200);
    const TEXT_WEAK: Color32 = Color32::from_gray(100);

    const ACCENT: Color32 = Color32::from_rgb(218, 223, 232);
    const SELECTION: Color32 = Color32::from_rgb(45, 104, 211);

    const WARN: Color32 = Color32::from_rgb(232, 181, 94);
    const ERROR: Color32 = Color32::from_rgb(244, 113, 113);
}

pub const CLEAR_COLOR: Color32 = Palette::BG_BASE;

pub fn install(ctx: &egui::Context) {
    ctx.global_style_mut(|style| {
        style.visuals = visuals();
        style.interaction.selectable_labels = false;
        style.url_in_tooltip = true;
        apply_spacing(style);
    });
}

fn visuals() -> egui::Visuals {
    let mut v = egui::Visuals::dark();

    v.dark_mode = true;

    v.panel_fill = Palette::BG_PANEL;
    v.window_fill = Palette::BG_BASE;
    v.extreme_bg_color = Palette::BG_DEEP;
    v.faint_bg_color = Palette::BG_FAINT;
    v.code_bg_color = Palette::BG_DEEP;

    v.warn_fg_color = Palette::WARN;
    v.error_fg_color = Palette::ERROR;
    v.hyperlink_color = Palette::ACCENT;
    v.weak_text_color = Some(Palette::TEXT_WEAK);

    v.window_stroke = Stroke::new(1.0_f32, Palette::BORDER);
    v.window_corner_radius = CornerRadius::same(7);
    v.menu_corner_radius = CornerRadius::same(5);
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(110),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(90),
    };

    v.selection.bg_fill = Palette::SELECTION;
    v.selection.stroke = Stroke::new(1.0_f32, Palette::ACCENT);

    v.slider_trailing_fill = true;

    let r = CornerRadius::same(4);
    let w = &mut v.widgets;

    w.noninteractive.bg_fill = Palette::BG_PANEL;
    w.noninteractive.weak_bg_fill = Palette::BG_PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT);
    w.noninteractive.corner_radius = r;

    w.inactive.bg_fill = Palette::SURFACE;
    w.inactive.weak_bg_fill = Palette::SURFACE;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT);
    w.inactive.corner_radius = r;

    w.hovered.bg_fill = Palette::SURFACE_HOVER;
    w.hovered.weak_bg_fill = Palette::SURFACE_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER_STRONG);
    w.hovered.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT_STRONG);
    w.hovered.corner_radius = r;

    w.active.bg_fill = Palette::SURFACE_ACTIVE;
    w.active.weak_bg_fill = Palette::SURFACE_ACTIVE;
    w.active.bg_stroke = Stroke::new(1.0_f32, Palette::ACCENT);
    w.active.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT_STRONG);
    w.active.corner_radius = r;

    w.open.bg_fill = Palette::SURFACE_HOVER;
    w.open.weak_bg_fill = Palette::SURFACE_HOVER;
    w.open.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER_STRONG);
    w.open.fg_stroke = Stroke::new(1.0_f32, Palette::TEXT_STRONG);
    w.open.corner_radius = r;

    v
}

fn apply_spacing(style: &mut egui::Style) {
    let spacing = &mut style.spacing;
    spacing.button_padding.x = 6.0;
    spacing.menu_margin = egui::Margin::same(6);

    spacing.scroll.bar_width = 9.0;
    spacing.scroll.floating = true;
    spacing.scroll.floating_allocated_width = 0.0;
}
