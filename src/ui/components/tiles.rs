use crate::ui::pane::StateSnapshot;

pub struct TreePane<P: Pane> {
    tree: egui_tiles::Tree<P>,
}

impl<P: Pane> TreePane<P> {
    pub fn new(id: impl Into<egui::Id>) -> Self {
        Self {
            tree: egui_tiles::Tree::empty(id),
        }
    }

    pub fn ui(&mut self, state: &mut StateSnapshot, ui: &mut egui::Ui) {
        self.tree.ui(state, ui);
    }

    pub fn add_pane(&mut self, inspector_pane: P)
    where
        P: PartialEq,
    {
        if utils::focus_pane_if_present(&mut self.tree, &inspector_pane) {
            return;
        }
        let child = self.tree.tiles.insert_pane(inspector_pane);
        utils::add_pane_to_tile_tree(&mut self.tree, child);
    }
}

pub trait Pane {
    fn pane_ui(
        &mut self,
        state: &mut StateSnapshot<'_>,
        ui: &mut egui::Ui,
    ) -> egui_tiles::UiResponse;

    fn tab_title(&self, state: &StateSnapshot<'_>) -> egui::WidgetText;
}

impl<'a, P: Pane> egui_tiles::Behavior<P> for StateSnapshot<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut P,
    ) -> egui_tiles::UiResponse {
        pane.pane_ui(self, ui)
    }

    fn tab_title_for_pane(&mut self, pane: &P) -> egui::WidgetText {
        pane.tab_title(self)
    }

    fn is_tab_closable(&self, _tiles: &egui_tiles::Tiles<P>, _tile_id: egui_tiles::TileId) -> bool {
        true
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn on_tab_button(
        &mut self,
        tiles: &mut egui_tiles::Tiles<P>,
        tile_id: egui_tiles::TileId,
        button_response: egui::Response,
    ) -> egui::Response {
        if button_response.clicked_by(egui::PointerButton::Middle) {
            tiles.remove(tile_id);
        }
        button_response
    }

    fn tab_hover_cursor_icon(&self) -> egui::CursorIcon {
        egui::CursorIcon::PointingHand
    }
}

mod utils {
    use egui_tiles::{Container, Tile, TileId, Tiles, Tree};

    pub(super) fn first_tabs_tile_id<Pane>(tiles: &Tiles<Pane>, start: TileId) -> Option<TileId> {
        match tiles.get(start)? {
            Tile::Pane(_) => None,
            Tile::Container(Container::Tabs(_)) => Some(start),
            Tile::Container(Container::Linear(linear)) => linear
                .children
                .iter()
                .find_map(|&child| first_tabs_tile_id(tiles, child)),
            Tile::Container(Container::Grid(grid)) => grid
                .children()
                .find_map(|&child| first_tabs_tile_id(tiles, child)),
        }
    }

    pub(super) fn add_pane_to_tile_tree<Pane>(tree: &mut Tree<Pane>, pane_tile: TileId) {
        let Some(root) = tree.root else {
            tree.root = Some(pane_tile);
            return;
        };

        match tree.tiles.get(root) {
            Some(Tile::Pane(_)) => {
                unreachable!("all_panes_must_have_tabs must be true")
            }
            Some(Tile::Container(Container::Tabs(_))) => {
                if let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get_mut(root) {
                    tabs.add_child(pane_tile);
                    tabs.set_active(pane_tile);
                }
            }
            Some(Tile::Container(Container::Linear(_)) | Tile::Container(Container::Grid(_))) => {
                if let Some(tabs_id) = first_tabs_tile_id(&tree.tiles, root) {
                    if let Some(Tile::Container(Container::Tabs(tabs))) =
                        tree.tiles.get_mut(tabs_id)
                    {
                        tabs.add_child(pane_tile);
                        tabs.set_active(pane_tile);
                    }
                } else {
                    let new_root = tree.tiles.insert_tab_tile(vec![root, pane_tile]);
                    tree.root = Some(new_root);
                }
            }
            None => {
                tree.root = Some(pane_tile);
            }
        }
    }

    pub(super) fn focus_pane_if_present<Pane: PartialEq>(
        tree: &mut Tree<Pane>,
        needle: &Pane,
    ) -> bool {
        let Some(tile_id) = tree.tiles.find_pane(needle) else {
            return false;
        };
        tree.make_active(|id, _| id == tile_id)
    }
}
