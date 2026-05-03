use egui::Widget;

use crate::{
    state::StateEvent,
    ui::{
        components::rename_text_edit::RenameTextEdit,
        rename::{RenameState, RenameTarget},
    },
};

pub fn renameable_label<'a>(
    default_label: impl Widget + 'a,
    pending_events: &'a mut Vec<StateEvent>,
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
                pending_events.push(StateEvent::ApplyRename(
                    rename_state.target.clone(),
                    rename_state.current_label.clone(),
                ));
            } else if escape_pressed || response.lost_focus() {
                pending_events.push(StateEvent::CancelRename);
            }
            
            response
        } else {
            ui.add(default_label)
        }
    }
}
