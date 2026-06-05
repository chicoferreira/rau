/// One physical wheel notch expressed in the unit we feed to the camera.
///
/// Matches egui's native `line_scroll_speed`, so on native (where a notch arrives
/// as a [`egui::MouseWheelUnit::Line`] of `1.0`) the camera feel is unchanged.
const POINTS_PER_NOTCH: f32 = 40.0;

/// Approximate pixel delta a browser reports for a single wheel notch. Used to
/// convert [`egui::MouseWheelUnit::Point`] (pixel) deltas back into notches so web
/// matches native instead of scrolling several times faster.
const PIXELS_PER_NOTCH: f32 = 100.0;

/// Reads raw `MouseWheel` events and normalizes every unit to a single, platform
/// independent scale.
pub fn read_raw_scroll(ui: &egui::Ui, viewport_height_points: f32) -> f32 {
    ui.input(|i| {
        i.events
            .iter()
            .filter_map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => Some(match unit {
                    egui::MouseWheelUnit::Line => delta.y * POINTS_PER_NOTCH,
                    egui::MouseWheelUnit::Point => delta.y / PIXELS_PER_NOTCH * POINTS_PER_NOTCH,
                    egui::MouseWheelUnit::Page => delta.y * viewport_height_points,
                }),
                _ => None,
            })
            .sum()
    })
}
