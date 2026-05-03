use egui::Response;
use egui_ltreeview::{Action, NodeBuilder, TreeView};

use crate::{file_storage::DirNode, project::file::ProjectFilePath, ui::pane::StateSnapshot};

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    if state.file_storage.is_polling() && state.file_storage.file_tree().is_empty() {
        return ui.spinner();
    }

    let (response, actions) = TreeView::new(ui.make_persistent_id("files_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            render_dir_nodes(
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
    dir_node: &DirNode,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, ProjectFilePath>,
    path: ProjectFilePath,
) {
    for (dir_name, dir_node) in dir_node.dirs() {
        let path = path.join(dir_name.clone());

        builder.node(NodeBuilder::dir(path.clone()).label(dir_name.clone()));

        render_dir_nodes(dir_node, builder, path);

        builder.close_dir();
    }

    for (file_name, file_path) in dir_node.files() {
        builder.node(NodeBuilder::leaf(file_path.clone()).label(file_name.clone()));
    }
}
