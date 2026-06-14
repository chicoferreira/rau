use crate::{
    project::{ProjectResource, ResourceKind, ViewportId},
    ui::{
        components::{
            inspector,
            resource_icons::{icon_tab_title, resource_kind_icon},
            tiles::Pane,
        },
        pane::StateSnapshot,
    },
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
            inspector::centered_error(ui, "This viewport no longer exists.");
            return egui_tiles::UiResponse::None;
        };

        let Some(texture_view_id) = viewport.texture_view_id() else {
            inspector::centered_error(
                ui,
                "No texture view is assigned to this viewport.\nAssign one to display its contents here.",
            );
            return egui_tiles::UiResponse::None;
        };

        let runtime_texture_view = state
            .runtime_project
            .texture_views
            .get_init(texture_view_id);

        let runtime_texture_view = match runtime_texture_view {
            Ok(Some(runtime)) => runtime,
            Ok(None) => {
                inspector::centered(ui, |ui| {
                    ui.add(egui::Spinner::new().size(ui.text_style_height(&egui::TextStyle::Body)));
                });
                return egui_tiles::UiResponse::None;
            }
            Err(err) => {
                inspector::centered_error(
                    ui,
                    format!("Couldn't initialize the texture view:\n{err}"),
                );
                return egui_tiles::UiResponse::None;
            }
        };

        let Some(egui_id) = runtime_texture_view.egui_id() else {
            inspector::centered_error(
                ui,
                "This texture view can't be displayed.\nOnly the Rgba8UnormSrgb format is supported in viewports.",
            );
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
        let label = state
            .project
            .viewports
            .get(self.viewport_id)
            .map(|viewport| viewport.label().to_string())
            .unwrap_or_default();
        icon_tab_title(resource_kind_icon(ResourceKind::Viewport), &label)
    }

    fn is_valid(&self, state: &StateSnapshot<'_>) -> bool {
        state.project.viewports.get(self.viewport_id).is_ok()
    }
}
