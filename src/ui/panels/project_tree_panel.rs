use egui::Response;
use egui_ltreeview::{Action, RowLayout, TreeView};
use egui_phosphor::regular;

use crate::{
    error::AppError,
    project::{
        BindGroupId, CameraId, ComputePassId, DimensionId, ModelId, PresentationId,
        ProjectResource, RenderPassId, RenderPipelineId, ResourceId, ResourceKind, SamplerId,
        ShaderId, TextureId, TextureViewId, UniformId, ViewportId,
    },
    ui::{
        components::{
            field, resource_icons,
            tree_node::{TreeNode, pending_create_node},
        },
        pane::StateSnapshot,
        rename::RenameTarget,
    },
    workspace::StateEvent,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
    RenderPipelineFolder,
    RenderPipeline(RenderPipelineId),
    RenderPassFolder,
    RenderPass(RenderPassId),
    ComputePassFolder,
    ComputePass(ComputePassId),
    Presentation(PresentationId),
}

fn pending_resource_node(
    state: &mut StateSnapshot,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, TreeNodeId>,
    kind: ResourceKind,
) {
    pending_create_node(
        builder,
        state.event_queue,
        state.rename_state,
        TreeNodeId::PendingCreate(kind),
        RenameTarget::CreateResource(kind),
    );
}

fn resource_icon(id: TreeNodeId) -> resource_icons::Icon {
    use TreeNodeId as N;
    let kind = match id {
        N::ShaderFolder | N::Shader(_) => ResourceKind::Shader,
        N::ViewportFolder | N::Viewport(_) => ResourceKind::Viewport,
        N::UniformFolder | N::Uniform(_) => ResourceKind::Uniform,
        N::BindGroupFolder | N::BindGroup(_) => ResourceKind::BindGroup,
        N::TextureFolder | N::Texture(_) => ResourceKind::Texture,
        N::TextureViewFolder | N::TextureView(_) => ResourceKind::TextureView,
        N::SamplerFolder | N::Sampler(_) => ResourceKind::Sampler,
        N::DimensionFolder | N::Dimension(_) => ResourceKind::Dimension,
        N::CameraFolder | N::Camera(_) => ResourceKind::Camera,
        N::ModelFolder | N::Model(_) => ResourceKind::Model,
        N::RenderPipelineFolder | N::RenderPipeline(_) => ResourceKind::RenderPipeline,
        N::RenderPassFolder | N::RenderPass(_) => ResourceKind::RenderPass,
        N::ComputePassFolder | N::ComputePass(_) => ResourceKind::ComputePass,
        N::Presentation(_) => ResourceKind::Presentation,
        N::PendingCreate(kind) => kind,
    };
    resource_icons::resource_kind_icon(kind)
}

fn node_resource_id(id: TreeNodeId) -> Option<ResourceId> {
    use TreeNodeId as N;
    Some(match id {
        N::Uniform(id) => id.into(),
        N::BindGroup(id) => id.into(),
        N::Viewport(id) => id.into(),
        N::Shader(id) => id.into(),
        N::Camera(id) => id.into(),
        N::Dimension(id) => id.into(),
        N::Sampler(id) => id.into(),
        N::Texture(id) => id.into(),
        N::TextureView(id) => id.into(),
        N::Model(id) => id.into(),
        N::RenderPipeline(id) => id.into(),
        N::RenderPass(id) => id.into(),
        N::ComputePass(id) => id.into(),
        N::Presentation(id) => id.into(),
        N::PendingCreate(_)
        | N::UniformFolder
        | N::BindGroupFolder
        | N::ViewportFolder
        | N::ShaderFolder
        | N::CameraFolder
        | N::DimensionFolder
        | N::SamplerFolder
        | N::TextureFolder
        | N::TextureViewFolder
        | N::ModelFolder
        | N::RenderPipelineFolder
        | N::RenderPassFolder
        | N::ComputePassFolder => return None,
    })
}

fn resource_folder(id: TreeNodeId, label: &str) -> TreeNode<'_, TreeNodeId> {
    let color = resource_icon(id).color;
    TreeNode::folder(id, label).with_closer_icons(regular::FOLDER, regular::FOLDER_OPEN, color)
}

fn resource_leaf<'a>(
    id: TreeNodeId,
    label: &'a str,
    error: Option<&AppError>,
) -> TreeNode<'a, TreeNodeId> {
    let node = TreeNode::new(id, label).with_icon(resource_icon(id));
    let node = match node_resource_id(id) {
        Some(id) => node.with_hover_text(egui::RichText::new(format!("{id:?}")).monospace()),
        None => node,
    };
    match error {
        Some(error) => {
            let message = error.to_string();
            node.with_label_color(|visuals| visuals.error_fg_color)
                .with_label_suffix(move |ui| {
                    let error_color = ui.visuals().error_fg_color;
                    ui.add_space(4.0);
                    ui.colored_label(error_color, regular::WARNING)
                        .on_hover_text(egui::RichText::new(&message).color(error_color));
                })
        }
        None => node,
    }
}

fn count_suffix(count: usize) -> impl FnMut(&mut egui::Ui) {
    move |ui| {
        field::weak_label(ui, format!(" ({count})"));
    }
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .row_layout(RowLayout::CompactAlignedLabels) // Align directory closers with leaf icons
        .override_indent(Some(25.0))
        .show(ui, |builder| {
            let presentation_error = state.runtime_project.get_error(PresentationId);
            resource_leaf(
                TreeNodeId::Presentation(PresentationId),
                "Presentation",
                presentation_error,
            )
            .with_event(
                "Inspect",
                StateEvent::InspectResource(PresentationId.into()),
            )
            .build_to(builder, state.event_queue, state.rename_state);

            resource_folder(TreeNodeId::RenderPassFolder, "Render Passes")
                .with_label_suffix(count_suffix(state.project.render_passes.len()))
                .with_event(
                    "Create New Render Pass",
                    StateEvent::CreateResource(ResourceKind::RenderPass),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::RenderPass);
            for (id, render_pass) in state.project.render_passes.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::RenderPass(id), render_pass.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::RenderPass(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Render Pass",
                        StateEvent::CreateResource(ResourceKind::RenderPass),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::ComputePassFolder, "Compute Passes")
                .with_label_suffix(count_suffix(state.project.compute_passes.len()))
                .with_event(
                    "Create New Compute Pass",
                    StateEvent::CreateResource(ResourceKind::ComputePass),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::ComputePass);
            for (id, compute_pass) in state.project.compute_passes.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::ComputePass(id), compute_pass.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::ComputePass(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Compute Pass",
                        StateEvent::CreateResource(ResourceKind::ComputePass),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::RenderPipelineFolder, "Render Pipelines")
                .with_label_suffix(count_suffix(state.project.render_pipelines.len()))
                .with_event(
                    "Create New Render Pipeline",
                    StateEvent::CreateResource(ResourceKind::RenderPipeline),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::RenderPipeline);
            for (id, r_pipeline) in state.project.render_pipelines.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::RenderPipeline(id), r_pipeline.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::RenderPipeline(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Render Pipeline",
                        StateEvent::CreateResource(ResourceKind::RenderPipeline),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::ShaderFolder, "Shaders")
                .with_label_suffix(count_suffix(state.project.shaders.len()))
                .with_event(
                    "Create New Shader",
                    StateEvent::CreateResource(ResourceKind::Shader),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Shader);
            for (id, shader) in state.project.shaders.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Shader(id), shader.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Shader(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Shader",
                        StateEvent::CreateResource(ResourceKind::Shader),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::BindGroupFolder, "Bind Groups")
                .with_label_suffix(count_suffix(state.project.bind_groups.len()))
                .with_event(
                    "Create New Bind Group",
                    StateEvent::CreateResource(ResourceKind::BindGroup),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::BindGroup);
            for (id, bind_group) in state.project.bind_groups.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::BindGroup(id), bind_group.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::BindGroup(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Bind Group",
                        StateEvent::CreateResource(ResourceKind::BindGroup),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::UniformFolder, "Uniforms")
                .with_label_suffix(count_suffix(state.project.uniforms.len()))
                .with_event(
                    "Create New Uniform",
                    StateEvent::CreateResource(ResourceKind::Uniform),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Uniform);
            for (id, uniform) in state.project.uniforms.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Uniform(id), uniform.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Uniform(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Uniform",
                        StateEvent::CreateResource(ResourceKind::Uniform),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::TextureFolder, "Textures")
                .with_label_suffix(count_suffix(state.project.textures.len()))
                .with_event(
                    "Create New Texture",
                    StateEvent::CreateResource(ResourceKind::Texture),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Texture);
            for (id, texture) in state.project.textures.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Texture(id), texture.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_event("Derive Texture View", StateEvent::DeriveTextureView(id))
                    .with_event("Save as Image", StateEvent::DownloadTextureImage(id))
                    .with_rename_event("Rename", RenameTarget::Texture(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Texture",
                        StateEvent::CreateResource(ResourceKind::Texture),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::TextureViewFolder, "Texture Views")
                .with_label_suffix(count_suffix(state.project.texture_views.len()))
                .with_event(
                    "Create New Texture View",
                    StateEvent::CreateResource(ResourceKind::TextureView),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::TextureView);
            for (id, texture_view) in state.project.texture_views.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::TextureView(id), texture_view.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::TextureView(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Texture View",
                        StateEvent::CreateResource(ResourceKind::TextureView),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::SamplerFolder, "Samplers")
                .with_label_suffix(count_suffix(state.project.samplers.len()))
                .with_event(
                    "Create New Sampler",
                    StateEvent::CreateResource(ResourceKind::Sampler),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Sampler);
            for (id, sampler) in state.project.samplers.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Sampler(id), sampler.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Sampler(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Sampler",
                        StateEvent::CreateResource(ResourceKind::Sampler),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::ModelFolder, "Models")
                .with_label_suffix(count_suffix(state.project.models.len()))
                .with_event(
                    "Create New Model",
                    StateEvent::CreateResource(ResourceKind::Model),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Model);
            for (id, model) in state.project.models.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Model(id), model.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_event(
                        "Create Bind Groups from Materials",
                        StateEvent::OpenMaterialBindGroupsModal(id),
                    )
                    .with_rename_event("Rename", RenameTarget::Model(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Model",
                        StateEvent::CreateResource(ResourceKind::Model),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::CameraFolder, "Cameras")
                .with_label_suffix(count_suffix(state.project.cameras.len()))
                .with_event(
                    "Create New Camera",
                    StateEvent::CreateResource(ResourceKind::Camera),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Camera);
            for (id, camera) in state.project.cameras.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Camera(id), camera.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Camera(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Camera",
                        StateEvent::CreateResource(ResourceKind::Camera),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::ViewportFolder, "Viewports")
                .with_label_suffix(count_suffix(state.project.viewports.len()))
                .with_event(
                    "Create New Viewport",
                    StateEvent::CreateResource(ResourceKind::Viewport),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Viewport);
            for (id, viewport) in state.project.viewports.list_sorted() {
                let is_main_viewport = state.project.presentation.main_viewport() == Some(id);
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Viewport(id), viewport.label(), error)
                    .with_event("View", StateEvent::OpenViewport(id))
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Viewport(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event_if(
                        !is_main_viewport,
                        "Set as Main Viewport",
                        "Already set as main viewport",
                        StateEvent::SetMainViewport(id),
                    )
                    .with_separator()
                    .with_event(
                        "Create New Viewport",
                        StateEvent::CreateResource(ResourceKind::Viewport),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();

            resource_folder(TreeNodeId::DimensionFolder, "Dimensions")
                .with_label_suffix(count_suffix(state.project.dimensions.len()))
                .with_event(
                    "Create New Dimension",
                    StateEvent::CreateResource(ResourceKind::Dimension),
                )
                .build_to(builder, state.event_queue, state.rename_state);
            pending_resource_node(state, builder, ResourceKind::Dimension);
            for (id, dimension) in state.project.dimensions.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(TreeNodeId::Dimension(id), dimension.label(), error)
                    .with_event("Inspect", StateEvent::InspectResource(id.into()))
                    .with_rename_event("Rename", RenameTarget::Dimension(id))
                    .with_event("Delete", StateEvent::DeleteResource(id.into()))
                    .with_separator()
                    .with_event(
                        "Create New Dimension",
                        StateEvent::CreateResource(ResourceKind::Dimension),
                    )
                    .build_to(builder, state.event_queue, state.rename_state);
            }
            builder.close_dir();
        });

    for action in actions {
        if let Action::SetSelected(selected) = action {
            for node in selected {
                let event = match node {
                    TreeNodeId::Viewport(id) => StateEvent::OpenViewport(id),
                    TreeNodeId::Uniform(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::BindGroup(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Shader(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Camera(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Dimension(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Sampler(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Texture(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::TextureView(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Model(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::RenderPipeline(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::RenderPass(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::ComputePass(id) => StateEvent::InspectResource(id.into()),
                    TreeNodeId::Presentation(id) => StateEvent::InspectResource(id.into()),
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
                    | TreeNodeId::RenderPipelineFolder
                    | TreeNodeId::RenderPassFolder
                    | TreeNodeId::ComputePassFolder
                    | TreeNodeId::PendingCreate(_) => continue,
                };

                state.event_queue.add(event);
            }
        }
    }

    response
}
