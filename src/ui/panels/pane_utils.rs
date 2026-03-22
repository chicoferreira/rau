use egui_tiles::{Container, Tile, TileId, Tiles, Tree};

fn first_tabs_tile_id<Pane>(tiles: &Tiles<Pane>, start: TileId) -> Option<TileId> {
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
                if let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get_mut(tabs_id) {
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

pub(super) fn focus_pane_if_present<Pane: PartialEq>(tree: &mut Tree<Pane>, needle: &Pane) -> bool {
    let Some(tile_id) = tree.tiles.find_pane(needle) else {
        return false;
    };
    tree.make_active(|id, _| id == tile_id)
}
