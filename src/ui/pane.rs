use crate::{scene, viewport};

pub fn build_default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();
    let tabs: Vec<egui_tiles::TileId> = vec![
        tiles.insert_pane(Pane::DeviceInfo),
        tiles.insert_pane(Pane::Viewport),
    ];
    let root: egui_tiles::TileId = tiles.insert_horizontal_tile(tabs);

    egui_tiles::Tree::new("my_tree", root, tiles)
}

pub enum Pane {
    Viewport,
    DeviceInfo,
}

impl Pane {
    pub fn title(&self) -> &'static str {
        match self {
            Pane::Viewport => "Viewport",
            Pane::DeviceInfo => "Device Info",
        }
    }
}

pub struct Behavior<'a> {
    pub viewport: &'a mut viewport::Viewport<scene::Scene>,
    pub adapter_info: &'a wgpu::AdapterInfo,
    pub pending_events: &'a mut Vec<viewport::ViewportEvent>,
}

impl<'a> egui_tiles::Behavior<Pane> for Behavior<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        match pane {
            Pane::Viewport => {
                let events = self.viewport.ui(ui);
                self.pending_events.extend(events);
            }
            Pane::DeviceInfo => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::Grid::new("adapter_info")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Adapter:");
                            ui.label(&self.adapter_info.name);
                            ui.end_row();

                            ui.label("Renderer:");
                            ui.label(self.adapter_info.backend.to_str());
                            ui.end_row();

                            ui.label("Driver:");
                            ui.label(&self.adapter_info.driver);
                            ui.end_row();

                            ui.label("Driver Info:");
                            ui.label(&self.adapter_info.driver_info);
                            ui.end_row();
                        });
                });
            }
        };

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        pane.title().into()
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}
