use egui::Context;

use crate::{state::StateEvent, ui::pane::StateSnapshot};

const ERROR_PANEL_ID: &str = "error_panel_expanded";

fn panel_id() -> egui::Id {
    egui::Id::new(ERROR_PANEL_ID)
}

fn is_open(ctx: &Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(panel_id()).unwrap_or(false))
}

fn set_open(ctx: &Context, open: bool) {
    ctx.data_mut(|d| d.insert_temp(panel_id(), open));
}

fn toggle_open(ctx: &Context) {
    let current = is_open(ctx);
    set_open(ctx, !current);
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    let mut show_error_list = is_open(ui.ctx());

    if state.errors.is_empty() && show_error_list {
        set_open(ui.ctx(), false);
        show_error_list = false;
    }

    egui::Panel::bottom("status_panel")
        .resizable(false)
        .show_inside(ui, |ui| {
            status_bar_content(state, ui);
        });

    if show_error_list && !state.errors.is_empty() {
        egui::Panel::bottom("error_list_panel")
            .resizable(true)
            .default_size(200.0)
            .min_size(80.0)
            .show_inside(ui, |ui| {
                error_list_content(state, ui);
            });
    }
}

fn status_bar_content(state: &StateSnapshot, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if state.errors.is_empty() {
            ui.label(egui::RichText::new("No errors").color(ui.visuals().weak_text_color()));
        } else {
            let error_count = state.errors.len();
            let label = format!(
                "{} error{}",
                error_count,
                if error_count == 1 { "" } else { "s" }
            );
            let btn =
                egui::Button::new(egui::RichText::new(label).color(ui.visuals().error_fg_color))
                    .frame(false);
            if ui
                .add(btn)
                .on_hover_text("Toggle error list")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                toggle_open(ui.ctx());
            }
        }
    });
}

fn error_list_content(state: &mut StateSnapshot, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for error in state.errors {
                ui.horizontal_wrapped(|ui| {
                    let source_label = error
                        .source
                        .map(|s| state.project.label(s).unwrap_or("Unknown"))
                        .unwrap_or("Unknown Source");

                    let label_text = egui::RichText::new(format!("@{}", source_label))
                        .strong()
                        .underline()
                        .color(ui.visuals().warn_fg_color);

                    let response = ui.add(egui::Button::new(label_text).frame(false));
                    if let Some(source) = error.source {
                        if response
                            .on_hover_text("Click to inspect source")
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                        {
                            state
                                .pending_events
                                .push(StateEvent::InspectResource(source));
                        }
                    }

                    ui.label(
                        egui::RichText::new(error.error.to_string())
                            .color(ui.visuals().error_fg_color),
                    );
                });

                ui.separator();
            }
        });
}
