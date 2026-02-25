use std::{cell::RefCell, rc::Rc};

use egui::Label;
use egui_ltreeview::{NodeBuilder, NodeConfig};

use crate::{
    state::StateEvent,
    ui::{
        components::renameable_label::renameable_label,
        panels::project_tree_panel::TreeNodeId,
        rename::{RenameState, RenameTarget},
    },
};

pub struct ProjectLeafNode<'a> {
    tree_id: TreeNodeId,
    label: &'a str,
    inspect_event: Option<StateEvent>,
    delete_event: Option<StateEvent>,
    rename_target: Option<RenameTarget>,
    create_new_label: &'a str,
    create_event: Option<StateEvent>,
}

impl<'a> ProjectLeafNode<'a> {
    pub fn new(tree_id: TreeNodeId, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            inspect_event: None,
            delete_event: None,
            rename_target: None,
            create_new_label: "Create New",
            create_event: None,
        }
    }

    pub fn with_inspect_event(mut self, inspect_event: StateEvent) -> Self {
        self.inspect_event = Some(inspect_event);
        self
    }

    pub fn with_delete_event(mut self, delete_event: StateEvent) -> Self {
        self.delete_event = Some(delete_event);
        self
    }

    pub fn with_rename_target(mut self, rename_event: RenameTarget) -> Self {
        self.rename_target = Some(rename_event);
        self
    }

    pub fn with_create_event(
        mut self,
        create_event: StateEvent,
        create_new_label: &'a str,
    ) -> Self {
        self.create_event = Some(create_event);
        self.create_new_label = create_new_label;
        self
    }

    pub fn build(
        self,
        pending_events: &'a mut Vec<StateEvent>,
        rename_state: &'a mut Option<RenameState>,
    ) -> impl NodeConfig<TreeNodeId> + 'a {
        let pending_events = Rc::new(RefCell::new(pending_events));
        let context_pending_events = Rc::clone(&pending_events);
        let label_pending_events = Rc::clone(&pending_events);

        let rename_target_cm = self.rename_target.clone();

        NodeBuilder::leaf(self.tree_id)
            .label(self.label)
            .context_menu(move |ui| {
                ui.set_min_width(130.0);
                let mut pending_events = context_pending_events.borrow_mut();
                if let Some(inspect_event) = self.inspect_event.clone() {
                    if ui.button("Inspect").clicked() {
                        pending_events.push(inspect_event);
                    }
                }
                if let Some(delete_event) = self.delete_event.clone() {
                    if ui.button("Delete").clicked() {
                        pending_events.push(delete_event);
                    }
                }
                if let Some(rename_target) = rename_target_cm.clone() {
                    if ui.button("Rename").clicked() {
                        pending_events.push(StateEvent::StartRename(rename_target));
                    }
                }
                if let Some(create_event) = self.create_event.clone() {
                    ui.separator();
                    if ui.button(self.create_new_label).clicked() {
                        pending_events.push(create_event);
                    }
                }
            })
            .label_ui(move |ui| {
                let default_label = Label::new(self.label).selectable(false);

                if let Some(rename_target) = self.rename_target.clone() {
                    ui.add(renameable_label(
                        default_label,
                        label_pending_events.borrow_mut().as_mut(),
                        rename_state,
                        rename_target,
                    ));
                } else {
                    ui.add(default_label);
                }
            })
    }
}
