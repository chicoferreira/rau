use egui::Response;
use egui_ltreeview::{Action, TreeView};

use crate::{
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, FramePlanId, ModelId, ProjectResource,
        RenderPassId, ResourceKind, SamplerId, ShaderId, TextureId, TextureViewId, UniformId,
        ViewportId,
    },
    ui::{
        components::tree_node::{TreeNode, pending_create_node},
        pane::StateSnapshot,
        rename::RenameTarget,
    },
    workspace::StateEvent,
};

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TreeNodeId {
    PendingCreate(ResourceKind),
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
    ModelFolder,
    Model(ModelId),
    RenderPassFolder,
    RenderPass(RenderPassId),
    ComputePassFolder,
    ComputePass(ComputePassId),
    FramePlan(FramePlanId),
}

fn pending_resource_node(
    state: &mut StateSnapshot,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, TreeNodeId>,
    kind: ResourceKind,
) {
    pending_create_node(
        builder,
        state.pending_events,
        state.rename_state,
        TreeNodeId::PendingCreate(kind),
        RenameTarget::CreateResource(kind),
    );
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .show(ui, |builder| {
            TreeNode::folder(TreeNodeId::UniformFolder, "Uniforms")
                .with_event(
                    "Create New Uniform",
                    StateEvent::CreateResource(ResourceKind::Uniform),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Uniform);
            for (id, uniform) in state.project.uniforms.list_sorted() {
                TreeNode::new(TreeNodeId::Uniform(id), &uniform.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Uniform(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Uniform",
                        StateEvent::CreateResource(ResourceKind::Uniform),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::BindGroupFolder, "Bind Groups")
                .with_event(
                    "Create New Bind Group",
                    StateEvent::CreateResource(ResourceKind::BindGroup),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::BindGroup);
            for (id, bind_group) in state.project.bind_groups.list_sorted() {
                TreeNode::new(TreeNodeId::BindGroup(id), bind_group.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::BindGroup(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Bind Group",
                        StateEvent::CreateResource(ResourceKind::BindGroup),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ViewportFolder, "Viewports")
                .with_event(
                    "Create New Viewport",
                    StateEvent::CreateResource(ResourceKind::Viewport),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Viewport);
            for (id, viewport) in state.project.viewports.list_sorted() {
                TreeNode::new(TreeNodeId::Viewport(id), &viewport.label)
                    .with_event("View", StateEvent::OpenViewport(id))
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Viewport(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Viewport",
                        StateEvent::CreateResource(ResourceKind::Viewport),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ShaderFolder, "Shaders")
                .with_event(
                    "Create New Shader",
                    StateEvent::CreateResource(ResourceKind::Shader),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Shader);
            for (id, shader) in state.project.shaders.list_sorted() {
                TreeNode::new(TreeNodeId::Shader(id), &shader.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Shader(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Shader",
                        StateEvent::CreateResource(ResourceKind::Shader),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::CameraFolder, "Cameras")
                .with_event(
                    "Create New Camera",
                    StateEvent::CreateResource(ResourceKind::Camera),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Camera);
            for (id, camera) in state.project.cameras.list_sorted() {
                TreeNode::new(TreeNodeId::Camera(id), &camera.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Camera(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Camera",
                        StateEvent::CreateResource(ResourceKind::Camera),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::DimensionFolder, "Dimensions")
                .with_event(
                    "Create New Dimension",
                    StateEvent::CreateResource(ResourceKind::Dimension),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Dimension);
            for (id, dimension) in state.project.dimensions.list_sorted() {
                TreeNode::new(TreeNodeId::Dimension(id), &dimension.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Dimension(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Dimension",
                        StateEvent::CreateResource(ResourceKind::Dimension),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::SamplerFolder, "Samplers")
                .with_event(
                    "Create New Sampler",
                    StateEvent::CreateResource(ResourceKind::Sampler),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Sampler);
            for (id, sampler) in state.project.samplers.list_sorted() {
                TreeNode::new(TreeNodeId::Sampler(id), sampler.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Sampler(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Sampler",
                        StateEvent::CreateResource(ResourceKind::Sampler),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::TextureFolder, "Textures").build_to(
                builder,
                state.pending_events,
                state.rename_state,
            );
            for (id, texture) in state.project.textures.list_sorted() {
                TreeNode::new(TreeNodeId::Texture(id), texture.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Texture(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::TextureViewFolder, "Texture Views")
                .with_event(
                    "Create New Texture View",
                    StateEvent::CreateResource(ResourceKind::TextureView),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::TextureView);
            for (id, texture_view) in state.project.texture_views.list_sorted() {
                TreeNode::new(TreeNodeId::TextureView(id), texture_view.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::TextureView(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Texture View",
                        StateEvent::CreateResource(ResourceKind::TextureView),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ModelFolder, "Models")
                // .with_event("Create New Model", StateEvent::CreateModel)
                .build_to(builder, state.pending_events, state.rename_state);
            for (id, model) in state.project.models.list_sorted() {
                TreeNode::new(TreeNodeId::Model(id), &model.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Model(id))
                    // .with_event("Delete", StateEvent::DeleteModel(id))
                    // .with_separator()
                    // .with_event("Create New Texture View", StateEvent::CreateTextureView)
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::RenderPassFolder, "Render Passes").build_to(
                builder,
                state.pending_events,
                state.rename_state,
            );
            for (id, render_pass) in state.project.render_passes.list_sorted() {
                TreeNode::new(TreeNodeId::RenderPass(id), &render_pass.label)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::RenderPass(id))
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::folder(TreeNodeId::ComputePassFolder, "Compute Passes")
                .with_event(
                    "Create New Compute Pass",
                    StateEvent::CreateResource(ResourceKind::ComputePass),
                )
                .build_to(builder, state.pending_events, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::ComputePass);
            for (id, compute_pass) in state.project.compute_passes.list_sorted() {
                TreeNode::new(TreeNodeId::ComputePass(id), compute_pass.label())
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::ComputePass(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Compute Pass",
                        StateEvent::CreateResource(ResourceKind::ComputePass),
                    )
                    .build_to(builder, state.pending_events, state.rename_state);
            }
            builder.close_dir();

            TreeNode::new(TreeNodeId::FramePlan(FramePlanId), "Frame Plan")
                .with_event("Inspect", StateEvent::InspectResource(FramePlanId.into()))
                .build_to(builder, state.pending_events, state.rename_state);
        });

    for action in actions {
        match action {
            Action::SetSelected(selected) => {
                for node in selected {
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
                        TreeNodeId::Model(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::RenderPass(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::ComputePass(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::FramePlan(id) => Some(StateEvent::InspectResource(id.into())),
                        TreeNodeId::UniformFolder
                        | TreeNodeId::BindGroupFolder
                        | TreeNodeId::ViewportFolder
                        | TreeNodeId::ShaderFolder
                        | TreeNodeId::CameraFolder
                        | TreeNodeId::DimensionFolder
                        | TreeNodeId::SamplerFolder
                        | TreeNodeId::TextureFolder
                        | TreeNodeId::TextureViewFolder
                        | TreeNodeId::ModelFolder
                        | TreeNodeId::RenderPassFolder
                        | TreeNodeId::ComputePassFolder
                        | TreeNodeId::PendingCreate(_) => None,
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
