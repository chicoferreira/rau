use crate::{project::ViewportId, ui::pane::StateSnapshot};

pub struct ViewportTreePane {
    tree: egui_tiles::Tree<ViewportPane>,
}

impl Default for ViewportTreePane {
    fn default() -> Self {
        let tree = egui_tiles::Tree::empty("viewport_tree");

        Self { tree }
    }
}

impl ViewportTreePane {
    pub fn ui(&mut self, behavior: &mut StateSnapshot, ui: &mut egui::Ui) {
        self.tree.ui(behavior, ui);
    }

    pub fn add_viewport(&mut self, texture_id: ViewportId) {
        let child = self.tree.tiles.insert_pane(ViewportPane { texture_id });

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

struct ViewportPane {
    texture_id: ViewportId,
}

impl<'a> egui_tiles::Behavior<ViewportPane> for StateSnapshot<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut ViewportPane,
    ) -> egui_tiles::UiResponse {
        let viewport = self
            .project
            .viewports
            .get(pane.texture_id)
            .expect("texture must exist");

        let dimension = self
            .project
            .dimensions
            .get(viewport.dimension_id)
            .expect("deal with this later");

        let events =
            crate::ui::components::viewport::ui(ui, viewport.egui_id(), dimension.size.into());
        self.pending_events.extend(events);

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &ViewportPane) -> egui::WidgetText {
        self.project
            .viewports
            .get(pane.texture_id)
            .map(|texture| texture.label.as_str().into())
            .unwrap_or("Empty Viewport".into())
    }

    fn is_tab_closable(
        &self,
        _tiles: &egui_tiles::Tiles<ViewportPane>,
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
