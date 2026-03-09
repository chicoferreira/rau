use egui::Ui;

use crate::{
    key::KeyboardState,
    project::ViewportId,
    state::{self, StateEvent, ViewportEvent},
    ui,
};

pub fn ui(
    ui: &mut Ui,
    viewport_id: ViewportId,
    egui_texture_id: egui::TextureId,
    last_size: Option<ui::Size2d>,
) -> Vec<state::StateEvent> {
    let mut events = Vec::new();

    let size_points = ui.available_size().max(egui::Vec2::new(1.0, 1.0));
    let pixels_per_point = ui.ctx().pixels_per_point();

    let requested_width = (size_points.x * pixels_per_point).round() as u32;
    let requested_height = (size_points.y * pixels_per_point).round() as u32;
    let requested_size = ui::Size2d::new(requested_width, requested_height);

    if last_size != Some(requested_size) {
        events.push(StateEvent::ViewportEvent(
            viewport_id,
            ViewportEvent::Resize {
                size: requested_size,
            },
        ));
    }

    let sized_texture = egui::load::SizedTexture::new(egui_texture_id, size_points);
    let image = egui::Image::new(sized_texture).sense(egui::Sense::click_and_drag());

    let response = ui.add(image);

    if response.clicked() || response.drag_started() {
        response.request_focus();
        events.push(StateEvent::ViewportEvent(viewport_id, ViewportEvent::Focus));
    }

    let keyboard_state = if response.has_focus() {
        ui.input(|input| KeyboardState::from_egui_input(input))
    } else {
        // TODO: remove me once camera gets refactored
        KeyboardState::empty()
    };

    let prev_keyboard_state = get_last_sent_keyboard_state(ui, viewport_id);
    if prev_keyboard_state.as_ref() != Some(&keyboard_state) {
        set_last_sent_keyboard_state(ui, viewport_id, keyboard_state.clone());

        events.push(StateEvent::ViewportEvent(
            viewport_id,
            ViewportEvent::KeyboardKeys { keyboard_state },
        ));
    }

    if response.dragged() {
        let delta_points = ui.input(|i| i.pointer.delta());
        if delta_points.x != 0.0 || delta_points.y != 0.0 {
            let delta_px = delta_points * pixels_per_point;
            events.push(StateEvent::ViewportEvent(
                viewport_id,
                ViewportEvent::Drag {
                    mouse_dx: delta_px.x,
                    mouse_dy: delta_px.y,
                },
            ));
        }
    }

    if response.hovered() {
        let scroll_points = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_points != 0.0 {
            events.push(StateEvent::ViewportEvent(
                viewport_id,
                ViewportEvent::Scroll {
                    delta_y_px: scroll_points * pixels_per_point,
                },
            ));
        }
    }

    events
}

fn get_last_sent_keyboard_state(ui: &mut Ui, viewport_id: ViewportId) -> Option<KeyboardState> {
    let data_id = last_sent_keyboard_state_data_id(viewport_id);
    ui.ctx().data(|d| d.get_temp(data_id))
}

fn set_last_sent_keyboard_state(ui: &mut Ui, viewport_id: ViewportId, state: KeyboardState) {
    let data_id = last_sent_keyboard_state_data_id(viewport_id);
    ui.ctx().data_mut(|d| d.insert_temp(data_id, state));
}

fn last_sent_keyboard_state_data_id(viewport_id: ViewportId) -> egui::Id {
    egui::Id::new(("viewport_keyboard_state", viewport_id))
}
