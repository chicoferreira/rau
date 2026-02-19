use crate::{
    project::{self, texture::TextureId},
    ui::pane::StateSnapshot,
};

pub struct ViewportTreePane {
    tree: egui_tiles::Tree<ViewportPane>,
    viewport_tile: egui_tiles::TileId,
}

impl Default for ViewportTreePane {
    fn default() -> Self {
        let mut tiles = egui_tiles::Tiles::default();

        let viewport_tile = tiles.insert_pane(ViewportPane { texture_id: None });

        let tree = egui_tiles::Tree::new("viewport_tree", viewport_tile, tiles);

        Self {
            tree,
            viewport_tile,
        }
    }
}

impl ViewportTreePane {
    pub fn ui(&mut self, behavior: &mut StateSnapshot, ui: &mut egui::Ui) {
        self.tree.ui(behavior, ui);
    }

    pub fn add_viewport(&mut self, texture_id: Option<project::texture::TextureId>) {
        let child = self.tree.tiles.insert_pane(ViewportPane { texture_id });

        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
            self.tree.tiles.get_mut(self.viewport_tile)
        {
            tabs.add_child(child);
            tabs.set_active(child);
            return;
        }

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
    texture_id: Option<TextureId>,
}

impl<'a> egui_tiles::Behavior<ViewportPane> for StateSnapshot<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut ViewportPane,
    ) -> egui_tiles::UiResponse {
        if let Some(texture_id) = pane.texture_id {
            let texture = self
                .project
                .get_texture(texture_id)
                .expect("texture must exist");

            let events = crate::ui::components::viewport::ui(ui, texture.egui_id(), texture.size());
            self.pending_events.extend(events);
        }

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &ViewportPane) -> egui::WidgetText {
        pane.texture_id
            .and_then(|id| self.project.get_texture(id))
            .map(|texture| texture.name().into())
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
