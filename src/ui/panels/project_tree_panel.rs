use egui::Response;
use egui_ltreeview::{Action, NodeBuilder, NodeConfig, TreeView};

use crate::{
    project::{
        bindgroup::BindGroupId,
        texture::TextureId,
        uniform::{Uniform, UniformId},
    },
    state::StateEvent,
    ui::pane::StateSnapshot,
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

    fn new_uniform_node<'a>(
        id: UniformId,
        uniform: &'a Uniform,
        pending_events: &'a mut Vec<StateEvent>,
    ) -> impl NodeConfig<TreeNodeId> + 'a {
        NodeBuilder::leaf(TreeNodeId::Uniform(id))
            .label(&uniform.label)
            .context_menu(move |ui| {
                if ui.button("Inspect").clicked() {
                    pending_events.push(StateEvent::InspectUniform(id));
                }
                if ui.button("Delete").clicked() {
                    pending_events.push(StateEvent::DeleteUniform(id));
                }
                ui.separator();
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
                let node = TreeNodeId::new_uniform_node(id, uniform, state.pending_events);
                builder.node(node);
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
