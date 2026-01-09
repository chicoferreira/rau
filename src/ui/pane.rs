use crate::{registry, ui};

pub fn build_default_tree() -> egui_tiles::Tree<Pane> {
    let mut tiles = egui_tiles::Tiles::default();

    let viewport_tile = tiles.insert_pane(Pane::Viewport);

    let inspector_tabs: Vec<egui_tiles::TileId> = vec![
        tiles.insert_pane(Pane::TextureInspector),
        tiles.insert_pane(Pane::DeviceInfo),
    ];
    let inspector_container = tiles.insert_tab_tile(inspector_tabs);

    let root = tiles.insert_horizontal_tile(vec![viewport_tile, inspector_container]);

    egui_tiles::Tree::new("my_tree", root, tiles)
}

pub enum Pane {
    Viewport,
    DeviceInfo,
    TextureInspector,
}

impl Pane {
    pub fn title(&self) -> &'static str {
        match self {
            Pane::Viewport => "Viewport",
            Pane::DeviceInfo => "Device Info",
            Pane::TextureInspector => "Textures",
        }
    }
}

pub struct Behavior<'a> {
    pub adapter_info: &'a wgpu::AdapterInfo,
    pub pending_events: &'a mut Vec<ui::viewport::ViewportEvent>,
    pub texture_registry: &'a mut registry::TextureRegistry,
    pub viewport_content: &'a mut Option<registry::TextureId>,
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
                if let Some(texture_id) = self.viewport_content {
                    let texture = self
                        .texture_registry
                        .get(*texture_id)
                        .expect("texture must exist");

                    let events = crate::ui::viewport::ui(ui, texture.egui_id(), texture.size());
                    self.pending_events.extend(events);
                }
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
            Pane::TextureInspector => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let selected = self
                        .viewport_content
                        .and_then(|id| self.texture_registry.get(id))
                        .map(|texture| texture.name())
                        .unwrap_or("Empty");

                    egui::ComboBox::from_label("Viewport Content")
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(self.viewport_content, None, "Empty");
                            for (id, element) in self.texture_registry.list() {
                                ui.selectable_value(
                                    self.viewport_content,
                                    Some(id),
                                    element.name(),
                                );
                            }
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
