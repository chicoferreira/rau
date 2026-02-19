use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    project::{bindgroup::BindGroupId, texture::TextureId, uniform::UniformId},
    ui::{
        pane::StateSnapshot,
        panels::{
            inspector_pane::{InspectorPane, InspectorTreePane},
            viewport_pane::ViewportTreePane,
        },
    },
};

#[derive(Clone, PartialEq, Eq, Hash)]
enum TreeNodeId {
    UniformFolder,
    Uniform(UniformId),
    BindGroupFolder,
    BindGroup(BindGroupId),
    ViewportFolder,
    Viewport(TextureId),
}

pub fn ui(
    state: &mut StateSnapshot,
    ui: &mut egui::Ui,
    inspector_tree_pane: &mut InspectorTreePane,
    viewport_tree_pane: &mut ViewportTreePane,
) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            builder.dir(TreeNodeId::UniformFolder, "Uniforms");
            for (id, uniform) in state.project.list_uniforms() {
                builder.leaf(TreeNodeId::Uniform(id), &uniform.label);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::BindGroupFolder, "Bind Groups");
            for (id, bind_group) in state.project.list_bind_groups() {
                builder.leaf(TreeNodeId::BindGroup(id), &bind_group.label);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::ViewportFolder, "Viewports");
            for (id, viewport) in state.project.list_textures() {
                builder.leaf(TreeNodeId::Viewport(id), viewport.name());
            }
            builder.close_dir();
        });

    for action in actions {
        match action {
            Action::Activate(activate) => {
                for node in activate.selected {
                    match node {
                        TreeNodeId::Uniform(id) => {
                            inspector_tree_pane.add_inspector_pane(InspectorPane::Uniform(id));
                        }
                        TreeNodeId::BindGroup(id) => {
                            inspector_tree_pane.add_inspector_pane(InspectorPane::BindGroup(id));
                        }
                        TreeNodeId::Viewport(id) => {
                            viewport_tree_pane.add_viewport(Some(id));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    response
}
