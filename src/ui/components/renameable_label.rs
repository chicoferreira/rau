use egui::Widget;

use crate::{
    ui::{
        components::rename_text_edit::RenameTextEdit,
        rename::{RenameState, RenameTarget},
    },
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

pub fn renameable_label<'a>(
    default_label: impl Widget + 'a,
    event_queue: &'a mut EventQueue<StateEvent>,
    rename_state: &'a mut Option<RenameState>,
    rename_target: RenameTarget,
) -> impl Widget + 'a {
    move |ui: &mut egui::Ui| {
        if let Some(rename_state) = rename_state
            && rename_state.target == rename_target
        {
            let text_edit_id = ui.id().with("rename").with(rename_target);

            let response = ui.add(RenameTextEdit::new(
                &mut rename_state.current_label,
                text_edit_id,
            ));

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            let escape_pressed = ui.input(|i| i.key_pressed(egui::Key::Escape));

            if response.lost_focus() && enter_pressed {
                event_queue.add(StateEvent::ApplyRename(
                    rename_state.target.clone(),
                    rename_state.current_label.clone(),
                ));
            } else if escape_pressed || response.lost_focus() {
                event_queue.add(StateEvent::CancelRename);
            }

            response
        } else {
            ui.add(default_label)
        }
    }
}
