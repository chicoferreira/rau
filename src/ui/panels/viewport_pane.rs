use crate::{
    project::{ProjectResource, ResourceKind, ViewportId},
    ui::{
        components::{
            resource_icons::{icon_tab_title, resource_kind_icon},
            tiles::Pane,
            viewport,
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
        viewport::ui(state, ui, self.viewport_id);
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
