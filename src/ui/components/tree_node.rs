use egui::Label;
use egui_ltreeview::{CloserState, NodeConfig, NodeId, TreeViewBuilder};
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
pub type NoContextMenu = fn(&mut ContextMenu<'_>);
/// Placeholder for nodes without a tooltip.
pub type NoHoverUi = fn(&mut egui::Ui);

pub struct TreeContext<'a> {
    pub event_queue: &'a mut EventQueue<StateEvent>,
    pub rename_state: &'a mut Option<RenameState>,
}

pub struct TreeNode<'a, T, F = NoContextMenu, H = NoHoverUi> {
    tree_id: T,
    label: &'a str,
    event_queue: &'a mut EventQueue<StateEvent>,
    rename_state: &'a mut Option<RenameState>,
    /// Resolves the label color from the current theme at render time.
    label_color: Option<LabelColorFn>,
    glyph: Option<NodeGlyph<'a>>,
    /// Child count rendered after the label.
    count: Option<usize>,
    /// Error whose badge is rendered after the label.
    error: Option<&'a AppError>,
    /// UI function filling the tooltip shown when hovering the node label.
    hover_ui: Option<H>,
    /// UI function to fill the context menu.
    context_menu: Option<F>,
    rename_target: Option<RenameTarget>,
    is_folder: bool,
}

pub fn pending_create_node<T>(
    builder: &mut TreeViewBuilder<'_, T>,
    ctx: &mut TreeContext<'_>,
    tree_id: T,
    rename_target: RenameTarget,
) where
    T: NodeId,
{
    let current_label = match ctx.rename_state.as_ref() {
        Some(rename_state) if rename_state.target == rename_target => {
            rename_state.current_label.clone()
        }
        _ => return,
    };

    TreeNode::new(ctx, tree_id, &current_label)
        .with_rename_target(rename_target)
        .build_to(builder);
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

impl<'a, T> TreeNode<'a, T, NoContextMenu, NoHoverUi>
where
    T: NodeId + 'a,
{
    pub fn new(ctx: &'a mut TreeContext<'_>, tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            event_queue: &mut *ctx.event_queue,
            rename_state: &mut *ctx.rename_state,
            label_color: None,
            glyph: None,
            count: None,
            error: None,
            hover_ui: None,
            context_menu: None,
            rename_target: None,
            is_folder: false,
        }
    }

    pub fn folder(ctx: &'a mut TreeContext<'_>, tree_id: T, label: &'a str) -> Self {
        Self {
            tree_id,
            label,
            event_queue: &mut *ctx.event_queue,
            rename_state: &mut *ctx.rename_state,
            label_color: None,
            glyph: None,
            count: None,
            error: None,
            hover_ui: None,
            context_menu: None,
            rename_target: None,
            is_folder: true,
        }
    }
}

impl<'a, T, F, H> TreeNode<'a, T, F, H>
where
    T: NodeId + 'a,
    F: FnMut(&mut ContextMenu<'_>) + 'a,
    H: FnMut(&mut egui::Ui) + 'a,
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

    /// Fill the tooltip shown when hovering the node label.
    pub fn with_hover_ui<G>(self, hover_ui: Option<G>) -> TreeNode<'a, T, F, G>
    where
        G: FnMut(&mut egui::Ui) + 'a,
    {
        TreeNode {
            tree_id: self.tree_id,
            label: self.label,
            event_queue: self.event_queue,
            rename_state: self.rename_state,
            label_color: self.label_color,
            glyph: self.glyph,
            count: self.count,
            error: self.error,
            hover_ui,
            context_menu: self.context_menu,
            rename_target: self.rename_target,
            is_folder: self.is_folder,
        }
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

    pub fn with_context_menu<G>(self, context_menu: G) -> TreeNode<'a, T, G, H>
    where
        G: FnMut(&mut ContextMenu<'_>) + 'a,
    {
        TreeNode {
            tree_id: self.tree_id,
            label: self.label,
            event_queue: self.event_queue,
            rename_state: self.rename_state,
            label_color: self.label_color,
            glyph: self.glyph,
            count: self.count,
            error: self.error,
            hover_ui: self.hover_ui,
            context_menu: Some(context_menu),
            rename_target: self.rename_target,
            is_folder: self.is_folder,
        }
    }

    pub fn build_to(self, builder: &mut TreeViewBuilder<'_, T>) -> bool {
        builder.node(self)
    }
}

impl<T, F, H> NodeConfig<T> for TreeNode<'_, T, F, H>
where
    T: NodeId,
    F: FnMut(&mut ContextMenu<'_>),
    H: FnMut(&mut egui::Ui),
{
    fn id(&self) -> &T {
        &self.tree_id
    }

    fn is_dir(&self) -> bool {
        self.is_folder
    }

    fn has_custom_icon(&self) -> bool {
        matches!(self.glyph, Some(NodeGlyph::Icon(_)))
    }

    fn icon(&mut self, ui: &mut egui::Ui) {
        if let Some(NodeGlyph::Icon(icon)) = &self.glyph {
            ui.add(Label::new(
                egui::RichText::new(icon.glyph).color(icon.color),
            ));
        }
    }

    fn has_custom_closer(&self) -> bool {
        matches!(self.glyph, Some(NodeGlyph::Closer { .. }))
    }

    fn closer(&mut self, ui: &mut egui::Ui, closer_state: CloserState) {
        if let Some(NodeGlyph::Closer {
            closed,
            open,
            color,
        }) = &self.glyph
        {
            let glyph = if closer_state.is_open { open } else { closed };
            ui.add(Label::new(egui::RichText::new(*glyph).color(*color)));
        }
    }

    fn label(&mut self, ui: &mut egui::Ui) {
        let Self {
            label,
            label_color,
            glyph,
            count,
            error,
            hover_ui,
            rename_target,
            event_queue,
            rename_state,
            ..
        } = self;

        if glyph.is_some() {
            ui.add_space(2.0);
        }

        let mut label_text = egui::RichText::new(*label);
        if let Some(resolve) = *label_color {
            label_text = label_text.color(resolve(ui.visuals()));
        }
        let default_label = Label::new(label_text);

        let add_label = |ui: &mut egui::Ui| match &*rename_target {
            Some(rename_target) => ui.add(renameable_label(
                default_label,
                event_queue,
                rename_state,
                rename_target,
            )),
            None => ui.add(default_label),
        };

        let response = match (*count, *error) {
            (None, None) => add_label(ui),
            (count, error) => {
                ui.scope(|ui| {
                    ui.style_mut().spacing.item_spacing.x = 0.0;

                    add_label(ui);

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
                .response
            }
        };

        if let Some(hover_ui) = hover_ui {
            response.on_hover_ui(|ui| hover_ui(ui));
        }
    }

    fn has_context_menu(&self) -> bool {
        self.context_menu.is_some()
    }

    fn context_menu(&mut self, ui: &mut egui::Ui) {
        let Self {
            context_menu,
            event_queue,
            ..
        } = self;
        let Some(context_menu) = context_menu else {
            return;
        };

        let mut menu = ContextMenu { ui, event_queue };
        context_menu(&mut menu);
    }
}
