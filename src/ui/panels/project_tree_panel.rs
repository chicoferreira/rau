use egui::{Response, WidgetText};
use egui_ltreeview::{Action, NodeBuilder, NodeConfig, TreeView};
use std::hash::Hash;

use crate::{
    project::{BindGroupId, ShaderId, UniformId, ViewportId},
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
    Viewport(ViewportId),
    ShaderFolder,
    Shader(ShaderId),
}

impl TreeNodeId {
    fn new_folder_node<'a>(
        id: TreeNodeId,
        label: impl Into<WidgetText> + 'a,
        create_new_label: impl Into<WidgetText> + Clone + 'a,
        create_event: StateEvent,
        pending_events: &'a mut Vec<StateEvent>,
    ) -> impl NodeConfig<TreeNodeId> {
        NodeBuilder::dir(id).label(label).context_menu(move |ui| {
            ui.set_min_width(130.0);
            if ui.button(create_new_label.clone()).clicked() {
                pending_events.push(create_event.clone());
            }
        })
    }
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            builder.node(TreeNodeId::new_folder_node(
                TreeNodeId::UniformFolder,
                "Uniforms",
                "Create New Uniform",
                StateEvent::CreateUniform,
                state.pending_events,
            ));

            for (id, uniform) in state.project.uniforms.list() {
                let node = ProjectLeafNode::new(TreeNodeId::Uniform(id), &uniform.label)
                    .with_rename_target(RenameTarget::Uniform(id))
                    .with_inspect_event(StateEvent::InspectUniform(id))
                    .with_create_event(StateEvent::CreateUniform, "Create New Uniform")
                    .with_delete_event(StateEvent::DeleteUniform(id))
                    .build(state.pending_events, state.rename_state);
                builder.node(node);
            }
            builder.close_dir();

            builder.node(TreeNodeId::new_folder_node(
                TreeNodeId::BindGroupFolder,
                "Bind Groups",
                "Create New Bind Group",
                StateEvent::CreateBindGroup,
                state.pending_events,
            ));
            for (id, bind_group) in state.project.bind_groups.list() {
                let node = ProjectLeafNode::new(TreeNodeId::BindGroup(id), &bind_group.label)
                    .with_rename_target(RenameTarget::BindGroup(id))
                    .with_inspect_event(StateEvent::InspectBindGroup(id))
                    .with_create_event(StateEvent::CreateBindGroup, "Create New Bind Group")
                    .with_delete_event(StateEvent::DeleteBindGroup(id))
                    .build(state.pending_events, state.rename_state);
                builder.node(node);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::ViewportFolder, "Viewports");
            for (id, viewport) in state.project.viewports.list() {
                let node = ProjectLeafNode::new(TreeNodeId::Viewport(id), &viewport.label)
                    .with_rename_target(RenameTarget::Viewport(id))
                    .with_inspect_event(StateEvent::OpenViewport(id))
                    .build(state.pending_events, state.rename_state);

                builder.node(node);
            }
            builder.close_dir();

            builder.dir(TreeNodeId::ShaderFolder, "Shaders");
            for (id, shader) in state.project.shaders.list() {
                let node = ProjectLeafNode::new(TreeNodeId::Shader(id), &shader.label)
                    .with_rename_target(RenameTarget::Shader(id))
                    .with_inspect_event(StateEvent::InspectShader(id))
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
                        TreeNodeId::Shader(id) => {
                            state.pending_events.push(StateEvent::InspectShader(id));
                        }
                        TreeNodeId::UniformFolder
                        | TreeNodeId::BindGroupFolder
                        | TreeNodeId::ViewportFolder
                        | TreeNodeId::ShaderFolder => {}
                    }
                }
            }
            _ => {}
        }
    }

    response
}
