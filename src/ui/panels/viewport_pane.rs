use crate::{
    project::ViewportId,
    ui::{components::tiles::Pane, pane::StateSnapshot},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ViewportPane {
    pub viewport_id: ViewportId,
}

impl Pane for ViewportPane {
    fn pane_ui(
        &mut self,
        state: &mut StateSnapshot<'_>,
        ui: &mut egui::Ui,
    ) -> egui_tiles::UiResponse {
        let Ok(viewport) = state.project.viewports.get(self.viewport_id) else {
            ui.label("Viewport couldn't be found.");
            return egui_tiles::UiResponse::None;
        };

        let Some(texture_view) = viewport
            .texture_view_id
            .and_then(|texture_view_id| state.project.texture_views.get(texture_view_id).ok())
        else {
            ui.label("Viewport points to a non-existent texture view.");
            return egui_tiles::UiResponse::None;
        };

        let Some(egui_id) = texture_view.egui_id() else {
            ui.label("Viewport points to a texture view format other than Rgba8UnormSrgb.");
            return egui_tiles::UiResponse::None;
        };

        let events = crate::ui::components::viewport::ui(
            ui,
            self.viewport_id,
            egui_id,
            viewport.requested_ui_size,
        );
        state.pending_events.extend(events);

        egui_tiles::UiResponse::None
    }

    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        state
            .project
            .viewports
            .get(self.viewport_id)
            .map(|texture| texture.label.as_str().into())
            .unwrap_or("Unknown Viewport".into())
    }
}
