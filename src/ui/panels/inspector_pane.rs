use crate::{
    project::{bindgroup::BindGroupId, uniform::UniformId},
    ui::pane::StateSnapshot,
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
        let child = self.tree.tiles.insert_pane(inspector_pane);

        if let Some(root) = self.tree.root {
            match self.tree.tiles.get_mut(root) {
                Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) => {
                    tabs.add_child(child);
                    tabs.set_active(child);
                }
                Some(egui_tiles::Tile::Container(container)) => {
                    container.add_child(child);
                }
                Some(egui_tiles::Tile::Pane(_)) => {
                    let new_root = self.tree.tiles.insert_tab_tile(vec![root, child]);
                    if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
                        self.tree.tiles.get_mut(new_root)
                    {
                        tabs.set_active(child);
                    }
                    self.tree.root = Some(new_root);
                }
                None => {
                    log::warn!("Tree root points to a missing tile; cannot add viewport");
                }
            }
        } else {
            self.tree.root = Some(child);
        }
    }
}

#[derive(Debug)]
pub enum InspectorPane {
    Uniform(UniformId),
    BindGroup(BindGroupId),
}

impl<'a> egui_tiles::Behavior<InspectorPane> for StateSnapshot<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut InspectorPane,
    ) -> egui_tiles::UiResponse {
        match pane {
            InspectorPane::Uniform(uniform_id) => {
                self.uniform_inspector_ui(*uniform_id, ui);
            }
            InspectorPane::BindGroup(bind_group_id) => {
                self.bind_group_inspector_ui(*bind_group_id, ui);
            }
        };

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &InspectorPane) -> egui::WidgetText {
        match pane {
            InspectorPane::Uniform(id) => self
                .project
                .get_uniform(*id)
                .map(|u| u.label.clone())
                .unwrap_or(format!("Unknown {id:?}"))
                .into(),
            InspectorPane::BindGroup(id) => self
                .project
                .get_bind_group(*id)
                .map(|b| b.label.clone())
                .unwrap_or(format!("Unknown {id:?}"))
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
