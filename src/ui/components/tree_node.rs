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
    /// Resolves the label color from the current theme at render time.
    label_color: Option<Box<dyn Fn(&egui::Visuals) -> egui::Color32 + 'a>>,
    icon: Option<&'a str>,
    icon_color: Option<egui::Color32>,
    closer_icons: Option<(&'a str, &'a str)>,
    /// Extra content rendered after the label (e.g. a child count).
    label_suffix: Option<Box<dyn FnMut(&mut egui::Ui) + 'a>>,
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
    Action {
        label: &'a str,
        event: StateEvent,
    },
    DisabledAction {
        label: &'a str,
        reason: Option<&'a str>,
    },
}

impl<'a, T> TreeNode<'a, T>
where
    T: Clone + Eq + Hash + 'a,
{
    pub fn new(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            label_color: None,
            icon: None,
            icon_color: None,
            closer_icons: None,
            label_suffix: None,
            events: Vec::new(),
            rename_target: None,
            is_folder: false,
        }
    }

    pub fn folder(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            label_color: None,
            icon: None,
            icon_color: None,
            closer_icons: None,
            label_suffix: None,
            events: Vec::new(),
            rename_target: None,
            is_folder: true,
        }
    }

    pub fn with_icon(mut self, icon: &'a str, color: [u8; 3]) -> Self {
        self.icon = Some(icon);
        self.icon_color = Some(egui::Color32::from_rgb(color[0], color[1], color[2]));
        self
    }

    pub fn with_closer_icons(mut self, closed: &'a str, open: &'a str, color: [u8; 3]) -> Self {
        self.closer_icons = Some((closed, open));
        self.icon_color = Some(egui::Color32::from_rgb(color[0], color[1], color[2]));
        self
    }

    /// Render extra content after the label (e.g. a child count or badge).
    pub fn with_label_suffix(mut self, add: impl FnMut(&mut egui::Ui) + 'a) -> Self {
        self.label_suffix = Some(Box::new(add));
        self
    }

    /// Tint the label text with a color resolved from the theme at render time.
    pub fn with_label_color(
        mut self,
        color: impl Fn(&egui::Visuals) -> egui::Color32 + 'a,
    ) -> Self {
        self.label_color = Some(Box::new(color));
        self
    }

    pub fn with_event(mut self, label: &'a str, event: StateEvent) -> Self {
        self.events.push(ContextMenuEntity::Action { label, event });
        self
    }

    pub fn with_event_if(
        mut self,
        condition: bool,
        label: &'a str,
        reason: impl Into<Option<&'a str>>,
        event: StateEvent,
    ) -> Self {
        let reason = reason.into();
        let action = match condition {
            true => ContextMenuEntity::Action { label, event },
            false => ContextMenuEntity::DisabledAction { label, reason },
        };
        self.events.push(action);
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

        let node = match self.icon {
            Some(icon) => {
                let icon_color = self.icon_color;
                node.icon(move |ui| {
                    let mut text = egui::RichText::new(icon);
                    if let Some(color) = icon_color {
                        text = text.color(color);
                    }
                    ui.add(Label::new(text));
                })
            }
            None => node,
        };

        let node = match self.closer_icons {
            Some((closed, open)) => {
                let icon_color = self.icon_color;
                node.closer(move |ui, state| {
                    let glyph = if state.is_open { open } else { closed };
                    let mut text = egui::RichText::new(glyph);
                    if let Some(color) = icon_color {
                        text = text.color(color);
                    }
                    ui.add(Label::new(text));
                })
            }
            None => node,
        };

        let mut label_suffix = self.label_suffix;
        let mut node = node.label(self.label).label_ui(move |ui| {
            let has_glyph = self.icon.is_some() || self.closer_icons.is_some();
            if has_glyph {
                ui.add_space(2.0);
            }

            let mut label_text = egui::RichText::new(self.label);
            if let Some(resolve) = &self.label_color {
                label_text = label_text.color(resolve(ui.visuals()));
            }
            let default_label = Label::new(label_text);

            ui.scope(|ui| {
                ui.style_mut().spacing.item_spacing.x = 0.0;

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

                if let Some(suffix) = &mut label_suffix {
                    suffix(ui);
                }
            });
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
                        ContextMenuEntity::DisabledAction { label, reason } => {
                            let response = ui.add_enabled(false, egui::Button::new(*label));
                            if let Some(reason) = reason {
                                response.on_disabled_hover_text(*reason);
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
