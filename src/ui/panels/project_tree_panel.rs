use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    project::{
        BindGroupId, CameraId, DimensionId, SamplerId, ShaderId, TextureId, TextureViewId,
        UniformId, ViewportId,
    },
    state::StateEvent,
    ui::{components::tree_node::TreeNode, pane::StateSnapshot, rename::RenameTarget},
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
    CameraFolder,
    Camera(CameraId),
    DimensionFolder,
    Dimension(DimensionId),
    SamplerFolder,
    Sampler(SamplerId),
    TextureFolder,
    Texture(TextureId),
    TextureViewFolder,
    TextureView(TextureViewId),
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            TreeNode::folder(TreeNodeId::UniformFolder, "Uniforms")
                .with_event("Create New Uniform", StateEvent::CreateUniform)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, uniform) in state.project.uniforms.list() {
                TreeNode::new(TreeNodeId::Uniform(id), &uniform.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Uniform(id))
                    .with_event("Delete", StateEvent::DeleteUniform(id))
                    .with_separator()
                    .with_event("Create New Uniform", StateEvent::CreateUniform)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::BindGroupFolder, "Bind Groups")
                .with_event("Create New Bind Group", StateEvent::CreateBindGroup)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, bind_group) in state.project.bind_groups.list() {
                TreeNode::new(TreeNodeId::BindGroup(id), bind_group.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::BindGroup(id))
                    .with_event("Delete", StateEvent::DeleteBindGroup(id))
                    .with_separator()
                    .with_event("Create New Bind Group", StateEvent::CreateBindGroup)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ViewportFolder, "Viewports")
                .with_event("Create New Viewport", StateEvent::CreateViewport)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, viewport) in state.project.viewports.list() {
                TreeNode::new(TreeNodeId::Viewport(id), &viewport.label)
                    .with_event("View", StateEvent::OpenViewport(id))
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Viewport(id))
                    .with_event("Delete", StateEvent::DeleteViewport(id))
                    .with_separator()
                    .with_event("Create New Viewport", StateEvent::CreateViewport)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ShaderFolder, "Shaders")
                .with_event("Create New Shader", StateEvent::CreateShader)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, shader) in state.project.shaders.list() {
                TreeNode::new(TreeNodeId::Shader(id), &shader.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Shader(id))
                    .with_event("Delete", StateEvent::DeleteShader(id))
                    .with_separator()
                    .with_event("Create New Shader", StateEvent::CreateShader)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::CameraFolder, "Cameras")
                .with_event("Create New Camera", StateEvent::CreateCamera)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, camera) in state.project.cameras.list() {
                TreeNode::new(TreeNodeId::Camera(id), &camera.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Camera(id))
                    .with_event("Delete", StateEvent::DeleteCamera(id))
                    .with_separator()
                    .with_event("Create New Camera", StateEvent::CreateCamera)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::DimensionFolder, "Dimensions")
                .with_event("Create New Dimension", StateEvent::CreateDimension)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, dimension) in state.project.dimensions.list() {
                TreeNode::new(TreeNodeId::Dimension(id), &dimension.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Dimension(id))
                    .with_event("Delete", StateEvent::DeleteDimension(id))
                    .with_separator()
                    .with_event("Create New Dimension", StateEvent::CreateDimension)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::SamplerFolder, "Samplers")
                .with_event("Create New Sampler", StateEvent::CreateSampler)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, sampler) in state.project.samplers.list() {
                TreeNode::new(TreeNodeId::Sampler(id), sampler.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Sampler(id))
                    .with_event("Delete", StateEvent::DeleteSampler(id))
                    .with_separator()
                    .with_event("Create New Sampler", StateEvent::CreateSampler)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::TextureFolder, "Textures").build_to(
                builder,
                state.pending_events,
                state.rename_state,
            );
            for (id, texture) in state.project.textures.list() {
                TreeNode::new(TreeNodeId::Texture(id), texture.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Texture(id))
                    .with_event("Delete", StateEvent::DeleteTexture(id))
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::TextureViewFolder, "Texture Views")
                .with_event("Create New Texture View", StateEvent::CreateTextureView)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, texture_view) in state.project.texture_views.list() {
                TreeNode::new(TreeNodeId::TextureView(id), texture_view.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::TextureView(id))
                    .with_event("Delete", StateEvent::DeleteTextureView(id))
                    .with_separator()
                    .with_event("Create New Texture View", StateEvent::CreateTextureView)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();
        });

    for action in actions {
        match action {
            Action::Activate(activate) => {
                for node in activate.selected {
                    let event = match node {
                        TreeNodeId::Viewport(id) => Some(StateEvent::OpenViewport(id)),
                        TreeNodeId::Uniform(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::BindGroup(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::Shader(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::Camera(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::Dimension(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::Sampler(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::Texture(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::TextureView(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::UniformFolder
                        | TreeNodeId::BindGroupFolder
                        | TreeNodeId::ViewportFolder
                        | TreeNodeId::ShaderFolder
                        | TreeNodeId::CameraFolder
                        | TreeNodeId::DimensionFolder
                        | TreeNodeId::SamplerFolder
                        | TreeNodeId::TextureFolder
                        | TreeNodeId::TextureViewFolder => None,
                    };

                    if let Some(event) = event {
                        state.pending_events.push(event);
                    }
                }
            }
            _ => {}
        }
    }

    response
}
