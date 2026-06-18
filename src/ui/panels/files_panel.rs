use egui::Response;
use egui_ltreeview::{Action, DirPosition, DragAndDrop, RowLayout, TreeView, TreeViewState};

use crate::{
    project::paths::FilePath,
    ui::{
        components::{
            resource_icons::{self, FOLDER_COLOR},
            tree_node::{TreeNode, pending_create_node},
        },
        pane::StateSnapshot,
        rename::{RenameState, RenameTarget},
    },
    utils::{dir_node::DirNode, event_queue::EventQueue},
    workspace::StateEvent,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum FileTreeNodeId {
    Root,
    Folder(FilePath),
    File(FilePath),
    Pending(FilePath),
}

fn pending_file_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    event_queue: &mut EventQueue<StateEvent>,
    rename_state: &mut Option<RenameState>,
    parent_path: FilePath,
) {
    pending_create_node(
        builder,
        event_queue,
        rename_state,
        FileTreeNodeId::Pending(parent_path.clone()),
        RenameTarget::CreateFile(parent_path),
    );
}

fn pending_folder_node(
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    event_queue: &mut EventQueue<StateEvent>,
    rename_state: &mut Option<RenameState>,
    parent_path: FilePath,
) {
    pending_create_node(
        builder,
        event_queue,
        rename_state,
        FileTreeNodeId::Pending(parent_path.clone()),
        RenameTarget::CreateFolder(parent_path),
    );
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let project_name = state
        .file_storage
        .project_source()
        .project_name()
        .to_string();

    let Some(file_tree) = state.file_storage.file_tree() else {
        return ui.spinner();
    };

    let tree_view_id = ui.make_persistent_id("files_tree_view");
    let mut tree_view_state = TreeViewState::load(ui, tree_view_id).unwrap_or_default();
    open_pending_create_parent(&mut tree_view_state, state.rename_state.as_ref());

    let (response, actions) = TreeView::new(tree_view_id)
        .allow_multi_selection(false)
        // Align directory closers with leaf icons
        .row_layout(RowLayout::CompactAlignedLabels)
        .override_indent(Some(25.0))
        .show_state(ui, &mut tree_view_state, |builder| {
            let root_path = FilePath::default();
            let event_queue = &mut *state.event_queue;
            let rename_state = &mut *state.rename_state;

            TreeNode::folder(FileTreeNodeId::Root, &project_name)
                .with_closer_icons(
                    egui_phosphor::regular::FOLDER,
                    egui_phosphor::regular::FOLDER_OPEN,
                    FOLDER_COLOR,
                )
                .with_event("Create File", StateEvent::CreateFile(FilePath::default()))
                .with_event(
                    "Create Folder",
                    StateEvent::CreateFolder(FilePath::default()),
                )
                .with_separator()
                .with_event("Import File", StateEvent::ImportFile(FilePath::default()))
                .build_to(builder, event_queue, rename_state);

            pending_file_node(builder, event_queue, rename_state, root_path.clone());
            pending_folder_node(builder, event_queue, rename_state, root_path.clone());

            render_dir_nodes(event_queue, rename_state, file_tree, builder, root_path);

            builder.close_dir();
        });

    tree_view_state.store(ui, tree_view_id);

    for action in actions {
        match action {
            Action::Drag(action) => {
                if file_move_event(&action).is_none() {
                    action.remove_drop_marker(ui);
                }
            }
            Action::Move(action) => {
                if let Some(event) = file_move_event(&action) {
                    state.event_queue.add(event);
                }
            }
            Action::SetSelected(selected) => {
                for node in selected {
                    let path = match node {
                        FileTreeNodeId::Root
                        | FileTreeNodeId::Folder(_)
                        | FileTreeNodeId::Pending(_) => continue,
                        FileTreeNodeId::File(path) => path.clone(),
                    };
                    state.event_queue.open_file(path);
                }
            }
            Action::Activate(_) | Action::DragExternal(_) | Action::MoveExternal(_) => {}
        }
    }

    response
}

fn open_pending_create_parent(
    tree_view_state: &mut TreeViewState<FileTreeNodeId>,
    rename_state: Option<&RenameState>,
) {
    let path = match rename_state.map(|state| &state.target) {
        Some(RenameTarget::CreateFile(path)) | Some(RenameTarget::CreateFolder(path)) => path,
        _ => return,
    };

    tree_view_state.set_openness(FileTreeNodeId::Root, true);

    for ancestor in path.ancestors_inclusive() {
        tree_view_state.set_openness(FileTreeNodeId::Folder(ancestor), true);
    }
}

fn render_dir_nodes(
    event_queue: &mut EventQueue<StateEvent>,
    rename_state: &mut Option<RenameState>,
    dir_node: &DirNode,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, FileTreeNodeId>,
    path: FilePath,
) {
    for (dir_name, dir_node) in dir_node.dirs() {
        let path = path
            .join(dir_name.clone())
            .expect("a dir from list_entries is be valid");

        TreeNode::folder(FileTreeNodeId::Folder(path.clone()), dir_name)
            .with_closer_icons(
                egui_phosphor::regular::FOLDER,
                egui_phosphor::regular::FOLDER_OPEN,
                FOLDER_COLOR,
            )
            .with_event("Create File", StateEvent::CreateFile(path.clone()))
            .with_event("Create Folder", StateEvent::CreateFolder(path.clone()))
            .with_rename_event("Rename Folder", RenameTarget::FileOrFolder(path.clone()))
            .with_separator()
            .with_event("Delete Folder", StateEvent::DeleteFolder(path.clone()))
            .with_separator()
            .with_event("Import File", StateEvent::ImportFile(path.clone()))
            .build_to(builder, event_queue, rename_state);

        pending_file_node(builder, event_queue, rename_state, path.clone());
        pending_folder_node(builder, event_queue, rename_state, path.clone());

        render_dir_nodes(event_queue, rename_state, dir_node, builder, path);

        builder.close_dir();
    }

    for (file_name, file_path) in dir_node.files() {
        let Some(path) = file_path.parent() else {
            unreachable!("A file path can't be the root")
        };

        let file_node = TreeNode::new(FileTreeNodeId::File(file_path.clone()), file_name)
            .with_icon(resource_icons::file_icon(file_path))
            .with_event("Open File", StateEvent::OpenFile(file_path.clone()));

        #[cfg(target_arch = "wasm32")]
        let file_node =
            file_node.with_event("Download File", StateEvent::DownloadFile(file_path.clone()));

        file_node
            .with_event("Create File", StateEvent::CreateFile(path.clone()))
            .with_event("Create Folder", StateEvent::CreateFolder(path.clone()))
            .with_rename_event("Rename File", RenameTarget::FileOrFolder(file_path.clone()))
            .with_separator()
            .with_event_if(
                !file_path.is_project_json(),
                "Delete File",
                "You can't delete the project.json file",
                StateEvent::DeleteFile(file_path.clone()),
            )
            .with_separator()
            .with_event("Import File", StateEvent::ImportFile(path.clone()))
            .with_event("Replace File", StateEvent::ReplaceFile(file_path.clone()))
            .build_to(builder, event_queue, rename_state);
    }
}

fn file_move_event(action: &DragAndDrop<FileTreeNodeId>) -> Option<StateEvent> {
    let [source] = action.source.as_slice() else {
        return None;
    };

    let old_path = source_path(source)?;
    let target_parent = target_folder_path(&action.target)?;

    if !valid_drop_position(&action.position) {
        return None;
    }

    let file_name = old_path.file_name()?;
    let new_path = target_parent.join(file_name.to_string()).ok()?;

    Some(StateEvent::MoveFileSystemEntry { old_path, new_path })
}

fn source_path(node: &FileTreeNodeId) -> Option<FilePath> {
    match node {
        FileTreeNodeId::Folder(path) | FileTreeNodeId::File(path) => Some(path.clone()),
        FileTreeNodeId::Root | FileTreeNodeId::Pending(_) => None,
    }
}

fn target_folder_path(node: &FileTreeNodeId) -> Option<FilePath> {
    match node {
        FileTreeNodeId::Root => Some(FilePath::default()),
        FileTreeNodeId::Folder(path) => Some(path.clone()),
        FileTreeNodeId::File(path) => path.parent(),
        FileTreeNodeId::Pending(_) => None,
    }
}

fn valid_drop_position(position: &DirPosition<FileTreeNodeId>) -> bool {
    match position {
        DirPosition::First | DirPosition::Last => true,
        DirPosition::Before(node) | DirPosition::After(node) => {
            matches!(node, FileTreeNodeId::Folder(_) | FileTreeNodeId::File(_))
        }
    }
}
