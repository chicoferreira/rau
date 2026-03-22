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

        let events = crate::ui::components::viewport::ui(
            ui,
            self.viewport_id,
            viewport.egui_id(),
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
