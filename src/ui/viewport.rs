use crate::ui;

pub enum ViewportEvent {
    Resize {
        size: ui::Size2d,
    },
    Scroll {
        delta_y_px: f32,
    },
    Drag {
        dx_px: f32,
        dy_px: f32,
    },
    Keyboard {
        key_code: winit::keyboard::KeyCode,
        element_state: winit::event::ElementState,
    },
    Frame {
        dt: instant::Duration,
    },
}

pub fn ui(
    ui: &mut egui::Ui,
    egui_texture_id: egui::TextureId,
    size: ui::Size2d,
) -> Vec<ViewportEvent> {
    let mut events = Vec::new();

    let size_points = ui.available_size().max(egui::Vec2::new(1.0, 1.0));
    let pixels_per_point = ui.ctx().pixels_per_point();

    let requested_width = (size_points.x * pixels_per_point).round() as u32;
    let requested_height = (size_points.y * pixels_per_point).round() as u32;

    if requested_width != size.width() || requested_height != size.height() {
        events.push(ViewportEvent::Resize {
            size: ui::Size2d::new(requested_width, requested_height),
        });
    }

    let sized_texture = egui::load::SizedTexture::new(egui_texture_id, size_points);
    let image = egui::Image::new(sized_texture).sense(egui::Sense::drag());

    let response = ui.add(image);

    if response.dragged_by(egui::PointerButton::Primary) {
        let delta_points = ui.input(|i| i.pointer.delta());
        if delta_points.x != 0.0 || delta_points.y != 0.0 {
            let delta_px = delta_points * pixels_per_point;
            events.push(ViewportEvent::Drag {
                dx_px: delta_px.x,
                dy_px: delta_px.y,
            });
        }
    }

    if response.hovered() {
        let scroll_points = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_points != 0.0 {
            events.push(ViewportEvent::Scroll {
                delta_y_px: scroll_points * pixels_per_point,
            });
        }
    }

    events
}
