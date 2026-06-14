use crate::{
    project::{ProjectResource, ViewportId},
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

        let Some(texture_view_id) = viewport.texture_view_id() else {
            ui.label("Please assign a texture view to this viewport.");
            return egui_tiles::UiResponse::None;
        };

        let runtime_texture_view = state
            .runtime_project
            .texture_views
            .get_init(texture_view_id);

        let runtime_texture_view = match runtime_texture_view {
            Ok(Some(runtime)) => runtime,
            Ok(None) => {
                ui.spinner();
                return egui_tiles::UiResponse::None;
            }
            Err(err) => {
                ui.colored_label(ui.visuals().error_fg_color, err.to_string());
                return egui_tiles::UiResponse::None;
            }
        };

        let Some(egui_id) = runtime_texture_view.egui_id() else {
            ui.label("Viewport points to a texture view format other than Rgba8UnormSrgb.");
            return egui_tiles::UiResponse::None;
        };

        let events = crate::ui::components::viewport::ui(
            ui,
            self.viewport_id,
            egui_id,
            viewport.requested_ui_size(),
        );
        state.event_queue.add_all(events);

        egui_tiles::UiResponse::None
    }

    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText {
        state
            .project
            .viewports
            .get(self.viewport_id)
            .map(|viewport| viewport.label().into())
            .unwrap_or("Unknown Viewport".into())
    }

    fn is_valid(&self, state: &StateSnapshot<'_>) -> bool {
        state.project.viewports.get(self.viewport_id).is_ok()
    }
}
