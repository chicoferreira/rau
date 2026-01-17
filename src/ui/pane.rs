use crate::{project, state, uniform};

pub struct AppTree {
    tree: egui_tiles::Tree<Pane>,
    viewport_container: egui_tiles::TileId,
}

impl AppTree {
    pub fn new_default() -> Self {
        let mut tiles = egui_tiles::Tiles::default();

        let viewport_tile = tiles.insert_pane(Pane::Viewport { texture_id: None });
        let viewport_container = tiles.insert_tab_tile(vec![viewport_tile]);

        let inspector_tabs: Vec<egui_tiles::TileId> = vec![
            tiles.insert_pane(Pane::TextureInspector),
            tiles.insert_pane(Pane::UniformInspector),
            tiles.insert_pane(Pane::DeviceInfo),
        ];
        let inspector_container = tiles.insert_tab_tile(inspector_tabs);

        let root = tiles.insert_horizontal_tile(vec![viewport_container, inspector_container]);

        let tree = egui_tiles::Tree::new("my_tree", root, tiles);

        Self {
            tree,
            viewport_container,
        }
    }

    pub fn ui(&mut self, behavior: &mut Behavior, ui: &mut egui::Ui) {
        self.tree.ui(behavior, ui);
    }

    pub fn add_viewport(&mut self, texture_id: Option<project::TextureId>) {
        let child = self.tree.tiles.insert_pane(Pane::Viewport { texture_id });

        if let Some(egui_tiles::Tile::Container(egui_tiles::Container::Tabs(tabs))) =
            self.tree.tiles.get_mut(self.viewport_container)
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

#[derive(Debug)]
pub enum Pane {
    Viewport {
        texture_id: Option<project::TextureId>,
    },
    DeviceInfo,
    UniformInspector,
    TextureInspector,
}

pub struct Behavior<'a> {
    pub adapter_info: &'a wgpu::AdapterInfo,
    pub pending_events: &'a mut Vec<state::StateEvent>,
    pub project: &'a mut project::Project,
    pub queue: &'a wgpu::Queue,
}

impl<'a> egui_tiles::Behavior<Pane> for Behavior<'a> {
    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        match pane {
            Pane::Viewport { texture_id } => {
                if let Some(texture_id) = texture_id {
                    let texture = self
                        .project
                        .get_texture(*texture_id)
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
                    for (id, element) in self.project.list_textures() {
                        if ui.button(element.name()).clicked() {
                            self.pending_events.push(state::StateEvent::AddViewport(id));
                        }
                    }
                });
            }
            Pane::UniformInspector => {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let uniform_ids = self
                        .project
                        .list_uniforms()
                        .map(|(id, _)| id)
                        .collect::<Vec<_>>();

                    if uniform_ids.is_empty() {
                        ui.label("No uniforms registered.");
                        return;
                    }

                    for uniform_id in uniform_ids {
                        let Some(uniform) = self.project.get_uniform_mut(uniform_id) else {
                            continue;
                        };

                        let mut new_data = uniform.data.clone();
                        let mut updated = false;

                        egui::CollapsingHeader::new(uniform.label.as_str())
                            .default_open(true)
                            .show(ui, |ui| {
                                for (index, field) in new_data.fields.iter_mut().enumerate() {
                                    match field {
                                        uniform::UniformField::Vec4(vec4) => {
                                            // TODO: change this to a label
                                            ui.label(format!("Vec4 #{index}"));
                                            ui.horizontal(|ui| {
                                                for value in vec4.iter_mut() {
                                                    updated |= ui
                                                        .add(
                                                            egui::DragValue::new(value).speed(0.01),
                                                        )
                                                        .changed();
                                                }
                                            });
                                        }
                                        uniform::UniformField::Mat4(mat4) => {
                                            // TODO: change this to a label
                                            ui.label(format!("Mat4 #{index}"));
                                            egui::Grid::new(format!(
                                                "uniform_{uniform_id:?}_mat4_{index}"
                                            ))
                                            .show(
                                                ui,
                                                |ui| {
                                                    for row in mat4.iter_mut() {
                                                        for value in row.iter_mut() {
                                                            updated |= ui
                                                                .add(
                                                                    egui::DragValue::new(value)
                                                                        .speed(0.01),
                                                                )
                                                                .changed();
                                                        }
                                                        ui.end_row();
                                                    }
                                                },
                                            );
                                        }
                                    }
                                    ui.add_space(8.0);
                                }
                            });

                        if updated {
                            uniform.update(self.queue, new_data);
                        }
                    }
                });
            }
        };

        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        match pane {
            Pane::Viewport { texture_id } => texture_id
                .and_then(|id| self.project.get_texture(id))
                .map(|texture| texture.name().into())
                .unwrap_or("Empty Viewport".into()),
            Pane::DeviceInfo => "Device Info".into(),
            Pane::TextureInspector => "Textures".into(),
            Pane::UniformInspector => "Uniforms".into(),
        }
    }

    fn is_tab_closable(
        &self,
        tiles: &egui_tiles::Tiles<Pane>,
        tile_id: egui_tiles::TileId,
    ) -> bool {
        matches!(
            tiles.get(tile_id),
            Some(egui_tiles::Tile::Pane(Pane::Viewport { texture_id: _ }))
        )
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}
