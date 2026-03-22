use crate::{
    project::{BindGroupId, CameraId, DimensionId, SamplerId, ShaderId, TextureViewId, UniformId},
    ui::{pane::StateSnapshot, panels::pane_utils},
};

pub struct InspectorTreePane {
    tree: egui_tiles::Tree<InspectorPane>,
}

impl Default for InspectorTreePane {
    fn default() -> Self {
        let tree = egui_tiles::Tree::empty("inspector_pane");

        Self { tree }
    }
}

impl InspectorTreePane {
    pub fn ui(&mut self, state: &mut StateSnapshot, ui: &mut egui::Ui) {
        self.tree.ui(state, ui);
    }

    pub fn add_inspector_pane(&mut self, inspector_pane: InspectorPane) {
        if pane_utils::focus_pane_if_present(&mut self.tree, &inspector_pane) {
            return;
        }
        let child = self.tree.tiles.insert_pane(inspector_pane);
        pane_utils::add_pane_to_tile_tree(&mut self.tree, child);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPane {
    Uniform(UniformId),
    BindGroup(BindGroupId),
    Shader(ShaderId),
    Camera(CameraId),
    Dimension(DimensionId),
    Sampler(SamplerId),
    TextureView(TextureViewId),
}

impl<'a> egui_tiles::Behavior<InspectorPane> for StateSnapshot<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut InspectorPane,
    ) -> egui_tiles::UiResponse {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                match pane {
                    InspectorPane::Uniform(uniform_id) => {
                        self.uniform_inspector_ui(*uniform_id, ui);
                    }
                    InspectorPane::BindGroup(bind_group_id) => {
                        self.bind_group_inspector_ui(*bind_group_id, ui);
                    }
                    InspectorPane::Shader(shader_id) => {
                        self.shader_inspector_ui(ui, *shader_id);
                    }
                    InspectorPane::Camera(camera_id) => {
                        self.camera_inspector_ui(ui, *camera_id);
                    }
                    InspectorPane::Dimension(dimension_id) => {
                        self.dimension_inspector_ui(ui, *dimension_id);
                    }
                    InspectorPane::Sampler(sampler_id) => {
                        self.sampler_inspector_ui(ui, *sampler_id);
                    }
                    InspectorPane::TextureView(texture_view_id) => {
                        self.texture_view_inspector_ui(ui, *texture_view_id)
                    }
                };
            });
        });

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &InspectorPane) -> egui::WidgetText {
        match pane {
            InspectorPane::Uniform(id) => self
                .project
                .uniforms
                .get(*id)
                .map(|u| u.label.clone())
                .unwrap_or(format!("Unknown {id:?}"))
                .into(),
            InspectorPane::BindGroup(id) => self
                .project
                .bind_groups
                .get(*id)
                .map(|b| b.label.clone())
                .unwrap_or(format!("Unknown {id:?}"))
                .into(),
            InspectorPane::Shader(shader_id) => self
                .project
                .shaders
                .get(*shader_id)
                .map(|s| s.label.clone())
                .unwrap_or(format!("Unknown {shader_id:?}"))
                .into(),
            InspectorPane::Camera(camera_id) => self
                .project
                .cameras
                .get(*camera_id)
                .map(|c| c.label.clone())
                .unwrap_or(format!("Unknown {camera_id:?}"))
                .into(),
            InspectorPane::Dimension(dimension_id) => self
                .project
                .dimensions
                .get(*dimension_id)
                .map(|d| d.label.clone())
                .unwrap_or(format!("Unknown {dimension_id:?}"))
                .into(),
            InspectorPane::Sampler(sampler_id) => self
                .project
                .samplers
                .get(*sampler_id)
                .map(|s| s.label().to_string())
                .unwrap_or(format!("Unknown {sampler_id:?}"))
                .into(),
            InspectorPane::TextureView(texture_view_id) => self
                .project
                .texture_views
                .get(*texture_view_id)
                .map(|s| s.label().to_string())
                .unwrap_or(format!("Unknown {texture_view_id:?}"))
                .into(),
        }
    }

    fn is_tab_closable(
        &self,
        _tiles: &egui_tiles::Tiles<InspectorPane>,
        _tile_id: egui_tiles::TileId,
    ) -> bool {
        true
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}
