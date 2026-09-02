use egui::Ui;

use crate::{
    project::{TextureViewId, ViewportId, resource::texture_view::previewable_formats_label},
    ui::{
        components::{field, inspector},
        pane::StateSnapshot,
        size::Size2d,
    },
    utils::{key::KeyboardState, raw_scroll},
    workspace::{StateEvent, ViewportEvent},
};

pub fn ui(state: &mut StateSnapshot<'_>, ui: &mut Ui, viewport_id: ViewportId) {
    puffin::profile_function!();

    let Ok(viewport) = state.project.viewports.get(viewport_id) else {
        inspector::centered_error(ui, "This viewport no longer exists.");
        return;
    };

    let last_size = viewport.requested_ui_size();

    let egui_texture_id = match egui_texture(state, viewport.texture_view_id()) {
        Ok(Some(egui_texture_id)) => egui_texture_id,
        Ok(None) => {
            field::centered(ui, field::spinner);
            return;
        }
        Err(message) => {
            inspector::centered_block(ui, |ui| {
                field::error_label(ui, message.as_str());
                ui.add_space(8.0);
                if ui.button("Open Inspector").clicked() {
                    state.event_queue.inspect_resource(viewport_id);
                }
            });
            return;
        }
    };

    let mut emit = |event: ViewportEvent| {
        state
            .event_queue
            .add(StateEvent::ViewportEvent(viewport_id, event));
    };

    let size_points = ui.available_size().max(egui::Vec2::new(1.0, 1.0));
    let pixels_per_point = ui.ctx().pixels_per_point();

    let requested_size = Size2d::new(
        (size_points.x * pixels_per_point).round() as u32,
        (size_points.y * pixels_per_point).round() as u32,
    );

    if last_size != Some(requested_size) {
        emit(ViewportEvent::Resize {
            size: requested_size,
        });
    }

    let sized_texture = egui::load::SizedTexture::new(egui_texture_id, size_points);
    let image = egui::Image::new(sized_texture).sense(egui::Sense::click_and_drag());

    let response = ui.add(image);

    if response.clicked() || response.drag_started() {
        response.request_focus();
        emit(ViewportEvent::Focus);
    }

    let keyboard_state = if response.has_focus() {
        ui.input(KeyboardState::from_egui_input)
    } else {
        KeyboardState::empty()
    };

    let prev_keyboard_state = get_last_sent_keyboard_state(ui, viewport_id);
    if prev_keyboard_state.as_ref() != Some(&keyboard_state) {
        set_last_sent_keyboard_state(ui, viewport_id, keyboard_state.clone());
        emit(ViewportEvent::KeyboardKeys { keyboard_state });
    }

    if response.dragged() {
        let delta_points = ui.input(|i| i.pointer.delta());
        if delta_points.x != 0.0 || delta_points.y != 0.0 {
            let delta_px = delta_points * pixels_per_point;
            emit(ViewportEvent::Drag {
                mouse_dx: delta_px.x,
                mouse_dy: delta_px.y,
            });
        }
    }

    if response.contains_pointer() {
        let scroll_delta = raw_scroll::read_raw_scroll(ui, size_points.y);
        if scroll_delta != 0.0 {
            emit(ViewportEvent::Scroll { scroll_delta });
        }
    }
}

fn egui_texture(
    state: &StateSnapshot<'_>,
    texture_view_id: Option<TextureViewId>,
) -> Result<Option<egui::TextureId>, String> {
    let Some(texture_view_id) = texture_view_id else {
        return Err("No texture view is assigned to this viewport.\n\
                    Assign one to display its contents here."
            .to_owned());
    };

    let runtime_texture_view = state
        .runtime_project
        .texture_views
        .get_init(texture_view_id)
        .map_err(|err| format!("Couldn't initialize the texture view:\n{err}"))?;

    let Some(runtime_texture_view) = runtime_texture_view else {
        return Ok(None);
    };

    runtime_texture_view.egui_id().map(Some).ok_or_else(|| {
        format!(
            "This texture view can't be displayed.\n\
             Only filterable RGBA formats ({}) are supported in viewports.",
            previewable_formats_label()
        )
    })
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
