use crate::ui::theme::{color, font_family, font_size};

const FRAME_CORNER_RADIUS: u8 = 5;
const FRAME_INNER_MARGIN: egui::Margin = egui::Margin::same(0);
const FRAME_STROKE: egui::Stroke = egui::Stroke {
    width: 1.0,
    color: color::BORDER_SUBTLE,
};

const HEADER_VERTICAL_PADDING: f32 = 6.0;
const HEADER_CORNER_RADIUS: egui::CornerRadius = egui::CornerRadius {
    nw: FRAME_CORNER_RADIUS,
    ne: FRAME_CORNER_RADIUS,
    sw: 0,
    se: 0,
};

const CONTENT_INNER_MARGIN: egui::Margin = egui::Margin::same(5);

pub fn field_container(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    let width = ui.available_width().max(1.0);

    egui::Frame::new()
        .fill(color::SURFACE_1)
        .stroke(FRAME_STROKE)
        .corner_radius(FRAME_CORNER_RADIUS)
        .inner_margin(FRAME_INNER_MARGIN)
        .show(ui, |ui| {
            ui.set_min_width(width);

            let content_item_spacing = ui.spacing().item_spacing;
            ui.spacing_mut().item_spacing.y = 0.0;

            draw_panel_header(ui, title);

            egui::Frame::new()
                .inner_margin(CONTENT_INNER_MARGIN)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = content_item_spacing;
                    add_contents(ui);
                });
        });
}

fn draw_panel_header(ui: &mut egui::Ui, title: &str) {
    let width = ui.available_width().max(1.0);

    let mut job = egui::text::LayoutJob::default();

    job.append(
        title,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::new(
                font_size::MD,
                egui::FontFamily::Name(font_family::PROPORTIONAL_SEMI_BOLD.into()),
            ),
            color: color::TEXT_PRIMARY,
            ..Default::default()
        },
    );

    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));

    let height = galley.size().y + HEADER_VERTICAL_PADDING * 2.0;

    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::hover());

    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, HEADER_CORNER_RADIUS, color::SURFACE_2);

    let galley_pos = rect.center() - galley.size() / 2.0;
    painter.galley(galley_pos, galley, color::TEXT_PRIMARY);
}
