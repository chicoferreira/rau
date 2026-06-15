use egui::{Frame, RichText, Stroke};
use egui_phosphor::regular;

use crate::{
    project::{Project, ResourceId, RuntimeProject},
    ui::{
        components::{inspector, resource_icons},
        pane::StateSnapshot,
    },
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

#[derive(Default)]
pub struct ErrorPanel {
    open: bool,
    errors: Vec<(ResourceId, String)>,
}

impl ErrorPanel {
    pub fn tick(&mut self, runtime_project: &RuntimeProject) {
        let current: Vec<_> = runtime_project
            .iter_errors()
            .map(|(id, error)| (id, error.to_string()))
            .collect();

        if current.iter().any(|id| !self.errors.contains(id)) {
            // Auto-open the panel if any resource becomes erroring that wasn't already erroring last frame.
            self.open = true;
        }

        self.errors = current;
    }

    pub fn status_indicator(&mut self, ui: &mut egui::Ui) {
        let caret = match self.open {
            true => regular::CARET_DOWN,
            false => regular::CARET_UP,
        };

        let (text, color) = match self.errors.len() {
            0 => (
                format!("{} No errors {caret}", regular::CHECK_CIRCLE),
                ui.visuals().weak_text_color(),
            ),
            count => (
                format!(
                    "{} {count} error{} {caret}",
                    regular::WARNING,
                    if count == 1 { "" } else { "s" },
                ),
                ui.visuals().error_fg_color,
            ),
        };

        let clicked = ui
            .add(egui::Button::new(RichText::new(text).color(color)).frame(false))
            .on_hover_text(match self.open {
                true => "Hide error list",
                false => "Show error list",
            })
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked();

        if clicked {
            self.open = !self.open;
        }
    }

    pub fn ui(&mut self, state: &mut StateSnapshot, ui: &mut egui::Ui) {
        if !self.open {
            return;
        }

        let open = &mut self.open;

        egui::Panel::bottom("error_list_panel")
            .resizable(true)
            .default_size(200.0)
            .min_size(80.0)
            .show_inside(ui, |ui| {
                error_list_content(open, ui, state.project, state.event_queue, &self.errors);
            });
    }
}

fn error_list_content(
    open: &mut bool,
    ui: &mut egui::Ui,
    project: &Project,
    event_queue: &mut EventQueue<StateEvent>,
    errors: &[(ResourceId, String)],
) {
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            ui.add(egui::Label::new(
                RichText::new("ERRORS").size(11.0).variation("wght", 600.0),
            ));
            inspector::weak_label(ui, format!(" ({})", errors.len()));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let close = ui
                .add(egui::Button::new(regular::X).frame(false))
                .on_hover_text("Hide error list")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked();

            if close {
                *open = false;
            }
        });
    });

    ui.add_space(2.0);

    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            if errors.is_empty() {
                inspector::centered(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{} No errors", regular::CHECK_CIRCLE))
                            .color(ui.visuals().weak_text_color()),
                    );
                });
                return;
            }

            for (id, error) in errors {
                error_card(ui, project, event_queue, *id, error);
                ui.add_space(4.0);
            }
        });
}

fn error_card(
    ui: &mut egui::Ui,
    project: &Project,
    event_queue: &mut EventQueue<StateEvent>,
    id: ResourceId,
    error: &str,
) {
    let error_color = ui.visuals().error_fg_color;

    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(Stroke::new(1.0, error_color.gamma_multiply(0.35)))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let source_label = project.label(id).unwrap_or("Unknown");
                let icon = resource_icons::resource_id_icon(id);
                let label_text = resource_icons::icon_text(ui, icon, source_label);

                let response = ui
                    .add(egui::Label::new(label_text).sense(egui::Sense::click()))
                    .on_hover_text("Click to open inspector")
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                if response.hovered() {
                    let rect = response.rect;
                    // Add underline when hovering over the label
                    ui.painter().hline(
                        rect.x_range(),
                        rect.bottom(),
                        Stroke::new(1.0, ui.visuals().text_color()),
                    );
                }

                if response.clicked() {
                    event_queue.inspect_resource(id);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(regular::COPY).frame(false))
                        .on_hover_text("Copy error message")
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        ui.ctx().copy_text(error.to_string());
                    }
                });
            });

            inspector::error_label(ui, error);
        });
}
