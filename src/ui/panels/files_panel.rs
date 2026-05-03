use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    file_storage::DirNode,
    project::file::ProjectFilePath,
    ui::{components::tree_node::TreeNode, pane::StateSnapshot},
};

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    if state.file_storage.is_polling() && state.file_storage.file_tree().is_empty() {
        return ui.spinner();
    }

    let (response, actions) = TreeView::new(ui.make_persistent_id("files_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            // TODO: add a root node with the name of the project
            render_dir_nodes(
                state,
                state.file_storage.file_tree(),
                builder,
                ProjectFilePath::default(),
            );
        });

    for action in actions {
        if let Action::SetSelected(_selected) = action {}
    }

    response
}

fn render_dir_nodes(
    state: &mut StateSnapshot,
    dir_node: &DirNode,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, ProjectFilePath>,
    path: ProjectFilePath,
) {
    for (dir_name, dir_node) in dir_node.dirs() {
        let path = path.join(dir_name.clone());

        TreeNode::folder(path.clone(), dir_name).build_to(
            builder,
            state.pending_events,
            state.rename_state,
        );

        render_dir_nodes(state, dir_node, builder, path);

        builder.close_dir();
    }

    for (file_name, file_path) in dir_node.files() {
        TreeNode::new(file_path.clone(), file_name).build_to(
            builder,
            state.pending_events,
            state.rename_state,
        );
    }
}
