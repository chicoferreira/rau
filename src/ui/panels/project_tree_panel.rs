use egui::Response;
use egui_ltreeview::{Action, NodeBuilder, NodeConfig, TreeView};
use std::hash::Hash;

use crate::{
    project::{bindgroup::BindGroupId, texture::TextureId, uniform::UniformId},
    state::StateEvent,
    ui::{
        components::project_leaf_node::ProjectLeafNode, pane::StateSnapshot, rename::RenameTarget,
    },
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TreeNodeId {
    UniformFolder,
    Uniform(UniformId),
    BindGroupFolder,
    BindGroup(BindGroupId),
    ViewportFolder,
    Viewport(TextureId),
}

impl TreeNodeId {
    fn new_uniform_folder_node(
        pending_events: &mut Vec<StateEvent>,
    ) -> impl NodeConfig<TreeNodeId> {
        NodeBuilder::dir(TreeNodeId::UniformFolder)
            .label("Uniforms")
            .context_menu(|ui| {
                if ui.button("Create New Uniform").clicked() {
                    pending_events.push(StateEvent::CreateUniform);
                }
            })
    }
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            builder.node(TreeNodeId::new_uniform_folder_node(state.pending_events));

            for (id, uniform) in state.project.list_uniforms() {
                let node = ProjectLeafNode::new(TreeNodeId::Uniform(id), id, &uniform.label)
                    .with_rename_target(RenameTarget::Uniform(id))
                    .with_inspect_event(StateEvent::InspectUniform(id))
                    .with_create_event(StateEvent::CreateUniform, "Create New Uniform")
                    .with_delete_event(StateEvent::DeleteUniform(id))
                    .build(state.pending_events, state.rename_state);
                builder.node(node);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::BindGroupFolder, "Bind Groups");
            for (id, bind_group) in state.project.list_bind_groups() {
                let node = ProjectLeafNode::new(TreeNodeId::BindGroup(id), id, &bind_group.label)
                    .with_rename_target(RenameTarget::BindGroup(id))
                    .with_inspect_event(StateEvent::InspectBindGroup(id))
                    // .with_create_event(StateEvent::CreateBindGroup)
                    // .with_delete_event(StateEvent::DeleteBindGroup(id))
                    .build(state.pending_events, state.rename_state);
                builder.node(node);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::ViewportFolder, "Viewports");
            for (id, viewport) in state.project.list_textures() {
                let node = ProjectLeafNode::new(TreeNodeId::Viewport(id), id, &viewport.name)
                    .with_rename_target(RenameTarget::Viewport(id))
                    .with_inspect_event(StateEvent::OpenViewport(id))
                    .build(state.pending_events, state.rename_state);

                builder.node(node);
            }
            builder.close_dir();
        });

    for action in actions {
        match action {
            Action::Activate(activate) => {
                for node in activate.selected {
                    match node {
                        TreeNodeId::Uniform(id) => {
                            state.pending_events.push(StateEvent::InspectUniform(id));
                        }
                        TreeNodeId::BindGroup(id) => {
                            state.pending_events.push(StateEvent::InspectBindGroup(id));
                        }
                        TreeNodeId::Viewport(id) => {
                            state.pending_events.push(StateEvent::OpenViewport(id));
                        }
                        TreeNodeId::UniformFolder
                        | TreeNodeId::BindGroupFolder
                        | TreeNodeId::ViewportFolder => {}
                    }
                }
            }
            _ => {}
        }
    }

    response
}
