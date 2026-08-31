use std::{cell::RefCell, rc::Rc};

use egui::Label;
use egui_ltreeview::{NodeBuilder, NodeConfig, NodeId, TreeViewBuilder};
use egui_phosphor::regular;

use crate::{
    error::AppError,
    ui::{
        components::{field, renameable_label::renameable_label, resource_icons::Icon},
        rename::{RenameState, RenameTarget},
    },
    utils::event_queue::EventQueue,
    workspace::StateEvent,
};

type LabelColorFn = fn(&egui::Visuals) -> egui::Color32;
/// Placeholder for nodes without a context menu.
type NoContextMenu = fn(&mut ContextMenu<'_>);

pub struct TreeNode<'a, T, F = NoContextMenu> {
    tree_id: T,
    label: &'a str,
    /// Resolves the label color from the current theme at render time.
    label_color: Option<LabelColorFn>,
    glyph: Option<NodeGlyph<'a>>,
    /// Child count rendered after the label.
    count: Option<usize>,
    /// Error whose badge is rendered after the label.
    error: Option<&'a AppError>,
    /// Tooltip shown when hovering the node label.
    hover_text: Option<egui::WidgetText>,
    /// UI function to fill the context menu.
    context_menu: Option<F>,
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
    T: NodeId,
{
    let current_label = match rename_state.as_ref() {
        Some(rename_state) if rename_state.target == rename_target => {
            rename_state.current_label.clone()
        }
        _ => return,
    };

    TreeNode::new(tree_id, &current_label)
        .with_rename_target(rename_target)
        .build_to(builder, event_queue, rename_state);
}

/// The glyph rendered before a node's label. A node has either a fixed leaf
/// icon or an expandable folder's open/closed pair, never both.
enum NodeGlyph<'a> {
    /// A fixed icon shown before the label (leaf nodes).
    Icon(Icon),
    /// Open/closed glyphs for an expandable folder, sharing one color.
    Closer {
        closed: &'a str,
        open: &'a str,
        color: egui::Color32,
    },
}

/// The context menu of an open node, rendered entry by entry.
pub struct ContextMenu<'a> {
    ui: &'a mut egui::Ui,
    event_queue: &'a mut EventQueue<StateEvent>,
}

impl ContextMenu<'_> {
    pub fn event(&mut self, label: &str, event: StateEvent) {
        if self.ui.button(label).clicked() {
            self.event_queue.add(event);
        }
    }

    pub fn event_if<'r>(
        &mut self,
        condition: bool,
        label: &str,
        reason: impl Into<Option<&'r str>>,
        event: StateEvent,
    ) {
        if condition {
            self.event(label, event);
            return;
        }

        let response = self.ui.add_enabled(false, egui::Button::new(label));
        if let Some(reason) = reason.into() {
            response.on_disabled_hover_text(reason);
        }
    }

    pub fn rename(&mut self, label: &str, rename_target: RenameTarget) {
        self.event(label, StateEvent::StartRename(rename_target));
    }

    pub fn separator(&mut self) {
        self.ui.separator();
    }
}

impl<'a, T> TreeNode<'a, T, NoContextMenu>
where
    T: NodeId + 'a,
{
    pub fn new(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            label_color: None,
            glyph: None,
            count: None,
            error: None,
            hover_text: None,
            context_menu: None,
            rename_target: None,
            is_folder: false,
        }
    }

    pub fn folder(tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            label_color: None,
            glyph: None,
            count: None,
            error: None,
            hover_text: None,
            context_menu: None,
            rename_target: None,
            is_folder: true,
        }
    }
}

impl<'a, T, F> TreeNode<'a, T, F>
where
    T: NodeId + 'a,
    F: FnMut(&mut ContextMenu<'_>) + 'a,
{
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.glyph = Some(NodeGlyph::Icon(icon));
        self
    }

    pub fn with_closer_icons(
        mut self,
        closed: &'a str,
        open: &'a str,
        color: egui::Color32,
    ) -> Self {
        self.glyph = Some(NodeGlyph::Closer {
            closed,
            open,
            color,
        });
        self
    }

    /// Render the number of children after the label.
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Render a warning badge after the label, with the error as its tooltip.
    pub fn with_error(mut self, error: &'a AppError) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_hover_text(mut self, text: impl Into<egui::WidgetText>) -> Self {
        self.hover_text = Some(text.into());
        self
    }

    /// Tint the label text with a color resolved from the theme at render time.
    pub fn with_label_color(mut self, color: LabelColorFn) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Allow the node to be renamed inline. Add a [`ContextMenu::rename`] entry
    /// to also offer it in the context menu.
    pub fn with_rename_target(mut self, rename_target: RenameTarget) -> Self {
        self.rename_target = Some(rename_target);
        self
    }

    pub fn with_context_menu<G>(self, context_menu: G) -> TreeNode<'a, T, G>
    where
        G: FnMut(&mut ContextMenu<'_>) + 'a,
    {
        TreeNode {
            tree_id: self.tree_id,
            label: self.label,
            label_color: self.label_color,
            glyph: self.glyph,
            count: self.count,
            error: self.error,
            hover_text: self.hover_text,
            context_menu: Some(context_menu),
            rename_target: self.rename_target,
            is_folder: self.is_folder,
        }
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

        let has_glyph = self.glyph.is_some();
        let node = match self.glyph {
            Some(NodeGlyph::Icon(icon)) => node.icon(move |ui| {
                ui.add(Label::new(
                    egui::RichText::new(icon.glyph).color(icon.color),
                ));
            }),
            Some(NodeGlyph::Closer {
                closed,
                open,
                color,
            }) => node.closer(move |ui, state| {
                let glyph = if state.is_open { open } else { closed };
                ui.add(Label::new(egui::RichText::new(glyph).color(color)));
            }),
            None => node,
        };

        let label = self.label;
        let label_color = self.label_color;
        let rename_target = self.rename_target;
        let count = self.count;
        let error = self.error;
        let hover_text = self.hover_text;
        let mut node = node.label(label).label_ui(move |ui| {
            if has_glyph {
                ui.add_space(2.0);
            }

            let mut label_text = egui::RichText::new(label);
            if let Some(resolve) = label_color {
                label_text = label_text.color(resolve(ui.visuals()));
            }
            let default_label = Label::new(label_text);

            let response = ui
                .scope(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 0.0;

                    if let Some(rename_target) = &rename_target {
                        let mut event_queue = label_event_queue.borrow_mut();
                        ui.add(renameable_label(
                            default_label,
                            &mut event_queue,
                            rename_state,
                            rename_target,
                        ));
                    } else {
                        ui.add(default_label);
                    }

                    if let Some(count) = count {
                        field::weak_label(ui, format!(" ({count})"));
                    }

                    if let Some(error) = error {
                        let error_color = ui.visuals().error_fg_color;
                        ui.add_space(4.0);
                        ui.colored_label(error_color, regular::WARNING)
                            .on_hover_ui(|ui| {
                                ui.set_max_width(ui.spacing().tooltip_width);
                                ui.label(egui::RichText::new(error.to_string()).color(error_color));
                            });
                    }
                })
                .response;

            if let Some(hover_text) = &hover_text {
                response.on_hover_text(hover_text.clone());
            }
        });

        if let Some(mut context_menu) = self.context_menu {
            node = node.context_menu(move |ui| {
                let mut event_queue = context_event_queue.borrow_mut();
                let mut menu = ContextMenu {
                    ui,
                    event_queue: &mut event_queue,
                };
                context_menu(&mut menu);
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
