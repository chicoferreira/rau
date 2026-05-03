use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    project::paths::FilePath,
    state::StateEvent,
    ui::{components::tree_node::TreeNode, pane::StateSnapshot, rename::RenameState},
    utils::dir_node::DirNode,
};

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let project_name = state
        .file_storage
        .project_identifier()
        .project_name()
        .to_string();

    let Some(file_tree) = state.file_storage.file_tree() else {
        return ui.spinner();
    };

    let (response, actions) = TreeView::new(ui.make_persistent_id("files_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            TreeNode::folder(FilePath::default(), &project_name).build_to(
                builder,
                state.pending_events,
                state.rename_state,
            );

            render_dir_nodes(
                &mut state.pending_events,
                &mut state.rename_state,
                file_tree,
                builder,
                FilePath::default(),
            );

            builder.close_dir();
        });

    for action in actions {
        if let Action::SetSelected(_selected) = action {}
    }

    response
}

fn render_dir_nodes(
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
    dir_node: &DirNode,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FilePath>,
    path: FilePath,
) {
    for (dir_name, dir_node) in dir_node.dirs() {
        let path = path.join(dir_name.clone());

        TreeNode::folder(path.clone(), dir_name).build_to(builder, pending_events, rename_state);

        render_dir_nodes(pending_events, rename_state, dir_node, builder, path);

        builder.close_dir();
    }

    for (file_name, file_path) in dir_node.files() {
        TreeNode::new(file_path.clone(), file_name).build_to(builder, pending_events, rename_state);
    }
}
