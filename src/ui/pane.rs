use crate::{
    app::AppEvent,
    file::file_storage::FileStorage,
    project::{Project, RuntimeProject},
    ui::{
        components::tiles::TreePane,
        panels::{
            error_panel, files_panel, inspector_pane::InspectorPane, menu_bar, project_tree_panel,
            viewport_pane::ViewportPane,
        },
        rename::RenameState,
    },
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

pub struct StateSnapshot<'a> {
    pub event_queue: &'a mut EventQueue<StateEvent>,
    pub app_event_queue: &'a mut EventQueue<AppEvent>,
    pub project: &'a mut Project,
    pub runtime_project: &'a mut RuntimeProject,
    pub rename_state: &'a mut Option<RenameState>,
    pub file_storage: &'a mut FileStorage,
    pub backend: wgpu::Backend,
}

impl StateSnapshot<'_> {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        inspector_tree_pane: &mut TreePane<InspectorPane>,
        viewport_tree_pane: &mut TreePane<ViewportPane>,
    ) {
        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            menu_bar::ui(self, ui);
        });

        error_panel::ui(self, ui);

        egui::Panel::left("left_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                let half_height = ui.available_height() * 0.5;

                egui::Panel::top("files_panel")
                    .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 0)))
                    .resizable(true)
                    .default_size(half_height)
                    .show_inside(ui, |ui| {
                        section_panel(ui, "File Explorer", |ui| files_panel::ui(self, ui));
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 0)))
                    .show_inside(ui, |ui| {
                        section_panel(ui, "Project Resources", |ui| {
                            project_tree_panel::ui(self, ui)
                        });
                    });
            });

        egui::Panel::right("inspector_tree_panel")
            .frame(egui::Frame::new().inner_margin(0))
            .resizable(true)
            .show_inside(ui, |ui| {
                inspector_tree_pane.ui(self, ui);
            });

        viewport_tree_pane.ui(self, ui);
    }
}

fn section_panel<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) {
    ui.take_available_height();

    let item_spacing = ui.spacing().item_spacing;
    ui.style_mut().spacing.item_spacing.y = 0.0;

    section_header(ui, title);

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.style_mut().spacing.item_spacing = item_spacing;
            add_contents(ui);
        });
}

fn section_header(ui: &mut egui::Ui, title: &str) {
    ui.scope(|ui| {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
        egui::Frame::new()
            .inner_margin(egui::Margin {
                top: 6,
                left: 8,
                bottom: 4,
                right: 4,
            })
            .show(ui, |ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new(title.to_uppercase())
                        .size(11.0)
                        .variation("wght", 600.0),
                ));
            });
    });
}
