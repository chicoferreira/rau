use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    project::paths::FilePath,
    state::StateEvent,
    ui::{
        components::tree_node::{TreeNode, pending_create_node},
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
    },
    utils::dir_node::DirNode,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum FileTreeNodeId {
    Path(FilePath),
    PendingCreateFile(FilePath),
    PendingCreateFolder(FilePath),
}

fn pending_file_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
    parent_path: FilePath,
) {
    pending_create_node(
        builder,
        pending_events,
        rename_state,
        FileTreeNodeId::PendingCreateFile(parent_path.clone()),
        RenameTarget::CreateFile(parent_path),
    );
}

fn pending_folder_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    pending_events: &mut Vec<StateEvent>,
    rename_state: &mut Option<RenameState>,
    parent_path: FilePath,
) {
    pending_create_node(
        builder,
        pending_events,
        rename_state,
        FileTreeNodeId::PendingCreateFolder(parent_path.clone()),
        RenameTarget::CreateFolder(parent_path),
    );
}

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
            let root_path = FilePath::default();
            let pending_events = &mut *state.pending_events;
            let rename_state = &mut *state.rename_state;

            TreeNode::folder(FileTreeNodeId::Path(root_path.clone()), &project_name)
                .with_event("Create File", StateEvent::CreateFile(FilePath::default()))
                .with_event(
                    "Create Folder",
                    StateEvent::CreateFolder(FilePath::default()),
                )
                .build_to(builder, pending_events, rename_state);

            pending_file_node(builder, pending_events, rename_state, root_path.clone());
            pending_folder_node(builder, pending_events, rename_state, root_path.clone());

            render_dir_nodes(pending_events, rename_state, file_tree, builder, root_path);

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
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    path: FilePath,
) {
    for (dir_name, dir_node) in dir_node.dirs() {
        let path = path.join(dir_name.clone());

        TreeNode::folder(FileTreeNodeId::Path(path.clone()), dir_name)
            .with_event("Create File", StateEvent::CreateFile(path.clone()))
            .with_event("Create Folder", StateEvent::CreateFolder(path.clone()))
            .build_to(builder, pending_events, rename_state);

        pending_file_node(builder, pending_events, rename_state, path.clone());
        pending_folder_node(builder, pending_events, rename_state, path.clone());

        render_dir_nodes(pending_events, rename_state, dir_node, builder, path);

        builder.close_dir();
    }

    for (file_name, file_path) in dir_node.files() {
        let Some(path) = file_path.parent() else {
            unreachable!("A file path can't be the root")
        };

        TreeNode::new(FileTreeNodeId::Path(file_path.clone()), file_name)
            .with_event("Create File", StateEvent::CreateFile(path))
            .with_rename_event("Rename File", RenameTarget::File(file_path.clone()))
            .with_event("Delete File", StateEvent::DeleteFile(file_path.clone()))
            .build_to(builder, pending_events, rename_state);
    }
}
