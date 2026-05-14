use std::{cell::RefCell, hash::Hash, rc::Rc};

use egui::Label;
use egui_ltreeview::{NodeBuilder, NodeConfig, TreeViewBuilder};

use crate::{
    ui::{
        components::renameable_label::renameable_label,
        rename::{RenameState, RenameTarget},
    },
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

pub struct TreeNode<'a, T> {
    tree_id: T,
    label: &'a str,
    events: Vec<ContextMenuEntity<'a>>,
    rename_target: Option<RenameTarget>,
    is_folder: bool,
}

pub fn pending_create_node<T>(
    builder: &mut TreeViewBuilder<'_, T>,
    event_queue: &mut EventQueue<StateEvent>,
    rename_state: &mut Option<RenameState>,
    tree_id: T,
    rename_target: RenameTarget,
) where
    T: Clone + Eq + Hash,
{
    let current_label = match rename_state.as_ref() {
        Some(rename_state) if rename_state.target == rename_target => {
            rename_state.current_label.clone()
        }
        _ => return,
    };

    TreeNode::new(tree_id, &current_label)
        .with_rename_target_only(rename_target)
        .build_to(builder, event_queue, rename_state);
}

enum ContextMenuEntity<'a> {
    Separator,
    Action { label: &'a str, event: StateEvent },
}

impl<'a, T> TreeNode<'a, T>
where
    T: Clone + Eq + Hash + 'a,
{
    pub fn new(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            events: Vec::new(),
            rename_target: None,
            is_folder: false,
        }
    }

    pub fn folder(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            events: Vec::new(),
            rename_target: None,
            is_folder: true,
        }
    }

    pub fn with_event(mut self, label: &'a str, event: StateEvent) -> Self {
        self.events.push(ContextMenuEntity::Action { label, event });
        self
    }

    pub fn with_separator(mut self) -> Self {
        self.events.push(ContextMenuEntity::Separator);
        self
    }

    pub fn with_rename_event(mut self, label: &'a str, rename_target: RenameTarget) -> Self {
        self.rename_target = Some(rename_target.clone());
        let event = StateEvent::StartRename(rename_target);
        self.with_event(label, event)
    }

    /// Inline rename UI without a context-menu entry (e.g. naming a resource before it exists).
    pub fn with_rename_target_only(mut self, rename_target: RenameTarget) -> Self {
        self.rename_target = Some(rename_target);
        self
    }

    fn into_node_config(
        self,
        event_queue: &'a mut EventQueue<StateEvent>,
        rename_state: &'a mut Option<RenameState>,
    ) -> impl NodeConfig<T> + 'a {
        let event_queue = Rc::new(RefCell::new(event_queue));
        let context_event_queue = Rc::clone(&event_queue);
        let label_event_queue = Rc::clone(&event_queue);
        let node = if self.is_folder {
            NodeBuilder::dir(self.tree_id)
        } else {
            NodeBuilder::leaf(self.tree_id)
        };

        let mut node = node.label(self.label).label_ui(move |ui| {
            let default_label = Label::new(self.label).selectable(false);

            if let Some(rename_target) = self.rename_target.clone() {
                let mut event_queue = label_event_queue.borrow_mut();
                ui.add(renameable_label(
                    default_label,
                    &mut **event_queue,
                    rename_state,
                    rename_target,
                ));
            } else {
                ui.add(default_label);
            }
        });

        if !self.events.is_empty() {
            node = node.context_menu(move |ui| {
                let mut event_queue = context_event_queue.borrow_mut();

                for event in self.events.iter() {
                    match event {
                        ContextMenuEntity::Separator => {
                            ui.separator();
                        }
                        ContextMenuEntity::Action { label, event } => {
                            if ui.button(*label).clicked() {
                                event_queue.add(event.clone());
                            }
                        }
                    }
                }
            });
        }

        node
    }

    pub fn build_to(
        self,
        builder: &mut TreeViewBuilder<'_, T>,
        event_queue: &'a mut EventQueue<StateEvent>,
        rename_state: &'a mut Option<RenameState>,
    ) -> bool {
        builder.node(self.into_node_config(event_queue, rename_state))
    }
}
