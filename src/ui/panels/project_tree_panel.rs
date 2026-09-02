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
            resource_icons,
            tree_node::{NoContextMenu, TreeContext, TreeNode, pending_create_node},
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
    ctx: &mut TreeContext<'_>,
    builder: &mut egui_ltreeview::TreeViewBuilder<'_, TreeNodeId>,
    kind: ResourceKind,
) {
    pending_create_node(
        builder,
        ctx,
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

fn resource_folder<'a>(
    ctx: &'a mut TreeContext<'_>,
    id: TreeNodeId,
    label: &'a str,
) -> TreeNode<'a, TreeNodeId> {
    let color = resource_icon(id).color;
    TreeNode::folder(ctx, id, label).with_closer_icons(regular::FOLDER, regular::FOLDER_OPEN, color)
}

fn resource_leaf<'a, R: ProjectResource>(
    ctx: &'a mut TreeContext<'_>,
    id: TreeNodeId,
    resource: &'a R,
    error: Option<&'a AppError>,
) -> TreeNode<'a, TreeNodeId, NoContextMenu, impl FnMut(&mut egui::Ui)> {
    let hover_ui = node_resource_id(id).map(|id| {
        move |ui: &mut egui::Ui| {
            ui.label(egui::RichText::new(format!("{id:?}")).monospace());
        }
    });

    let node = TreeNode::new(ctx, id, resource.label())
        .with_icon(resource_icon(id))
        .with_hover_ui(hover_ui);

    match error {
        Some(error) => node
            .with_label_color(|visuals| visuals.error_fg_color)
            .with_error(error),
        None => node,
    }
}

pub fn ui(state: &mut StateSnapshot, ui: &mut egui::Ui) -> Response {
    puffin::profile_function!();

    let (response, actions) = TreeView::new(ui.make_persistent_id("project_tree_view"))
        .allow_multi_selection(false)
        .row_layout(RowLayout::CompactAlignedLabels) // Align directory closers with leaf icons
        .override_indent(Some(25.0))
        .show(ui, |builder| {
            let ctx = &mut TreeContext {
                event_queue: state.event_queue,
                rename_state: state.rename_state,
            };

            let presentation_error = state.runtime_project.get_error(PresentationId);
            resource_leaf(
                ctx,
                TreeNodeId::Presentation(PresentationId),
                &state.project.presentation,
                presentation_error,
            )
            .with_context_menu(|menu| {
                menu.event(
                    "Inspect",
                    StateEvent::InspectResource(PresentationId.into()),
                );
            })
            .build_to(builder);

            resource_folder(ctx, TreeNodeId::RenderPassFolder, "Render Passes")
                .with_count(state.project.render_passes.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Render Pass",
                        StateEvent::CreateResource(ResourceKind::RenderPass),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::RenderPass);
            for (id, render_pass) in state.project.render_passes.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::RenderPass(id), render_pass, error)
                    .with_rename_target(RenameTarget::RenderPass(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::RenderPass(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Render Pass",
                            StateEvent::CreateResource(ResourceKind::RenderPass),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::ComputePassFolder, "Compute Passes")
                .with_count(state.project.compute_passes.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Compute Pass",
                        StateEvent::CreateResource(ResourceKind::ComputePass),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::ComputePass);
            for (id, compute_pass) in state.project.compute_passes.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::ComputePass(id), compute_pass, error)
                    .with_rename_target(RenameTarget::ComputePass(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::ComputePass(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Compute Pass",
                            StateEvent::CreateResource(ResourceKind::ComputePass),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::RenderPipelineFolder, "Render Pipelines")
                .with_count(state.project.render_pipelines.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Render Pipeline",
                        StateEvent::CreateResource(ResourceKind::RenderPipeline),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::RenderPipeline);
            for (id, r_pipeline) in state.project.render_pipelines.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::RenderPipeline(id), r_pipeline, error)
                    .with_rename_target(RenameTarget::RenderPipeline(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::RenderPipeline(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Render Pipeline",
                            StateEvent::CreateResource(ResourceKind::RenderPipeline),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::ShaderFolder, "Shaders")
                .with_count(state.project.shaders.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Shader",
                        StateEvent::CreateResource(ResourceKind::Shader),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Shader);
            for (id, shader) in state.project.shaders.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Shader(id), shader, error)
                    .with_rename_target(RenameTarget::Shader(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Shader(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Shader",
                            StateEvent::CreateResource(ResourceKind::Shader),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::BindGroupFolder, "Bind Groups")
                .with_count(state.project.bind_groups.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Bind Group",
                        StateEvent::CreateResource(ResourceKind::BindGroup),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::BindGroup);
            for (id, bind_group) in state.project.bind_groups.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::BindGroup(id), bind_group, error)
                    .with_rename_target(RenameTarget::BindGroup(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::BindGroup(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Bind Group",
                            StateEvent::CreateResource(ResourceKind::BindGroup),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::UniformFolder, "Uniforms")
                .with_count(state.project.uniforms.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Uniform",
                        StateEvent::CreateResource(ResourceKind::Uniform),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Uniform);
            for (id, uniform) in state.project.uniforms.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Uniform(id), uniform, error)
                    .with_rename_target(RenameTarget::Uniform(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Uniform(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Uniform",
                            StateEvent::CreateResource(ResourceKind::Uniform),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::TextureFolder, "Textures")
                .with_count(state.project.textures.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Texture",
                        StateEvent::CreateResource(ResourceKind::Texture),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Texture);
            for (id, texture) in state.project.textures.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Texture(id), texture, error)
                    .with_rename_target(RenameTarget::Texture(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.event("Derive Texture View", StateEvent::DeriveTextureView(id));
                        menu.event("Save as Image", StateEvent::DownloadTextureImage(id));
                        menu.rename("Rename", RenameTarget::Texture(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Texture",
                            StateEvent::CreateResource(ResourceKind::Texture),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::TextureViewFolder, "Texture Views")
                .with_count(state.project.texture_views.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Texture View",
                        StateEvent::CreateResource(ResourceKind::TextureView),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::TextureView);
            for (id, texture_view) in state.project.texture_views.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::TextureView(id), texture_view, error)
                    .with_rename_target(RenameTarget::TextureView(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::TextureView(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Texture View",
                            StateEvent::CreateResource(ResourceKind::TextureView),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::SamplerFolder, "Samplers")
                .with_count(state.project.samplers.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Sampler",
                        StateEvent::CreateResource(ResourceKind::Sampler),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Sampler);
            for (id, sampler) in state.project.samplers.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Sampler(id), sampler, error)
                    .with_rename_target(RenameTarget::Sampler(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Sampler(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Sampler",
                            StateEvent::CreateResource(ResourceKind::Sampler),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::ModelFolder, "Models")
                .with_count(state.project.models.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Model",
                        StateEvent::CreateResource(ResourceKind::Model),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Model);
            for (id, model) in state.project.models.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Model(id), model, error)
                    .with_rename_target(RenameTarget::Model(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.event(
                            "Create Bind Groups from Materials",
                            StateEvent::OpenMaterialBindGroupsModal(id),
                        );
                        menu.rename("Rename", RenameTarget::Model(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Model",
                            StateEvent::CreateResource(ResourceKind::Model),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::CameraFolder, "Cameras")
                .with_count(state.project.cameras.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Camera",
                        StateEvent::CreateResource(ResourceKind::Camera),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Camera);
            for (id, camera) in state.project.cameras.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Camera(id), camera, error)
                    .with_rename_target(RenameTarget::Camera(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Camera(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Camera",
                            StateEvent::CreateResource(ResourceKind::Camera),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::ViewportFolder, "Viewports")
                .with_count(state.project.viewports.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Viewport",
                        StateEvent::CreateResource(ResourceKind::Viewport),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Viewport);
            for (id, viewport) in state.project.viewports.list_sorted() {
                let is_main_viewport = state.project.presentation.main_viewport() == Some(id);
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Viewport(id), viewport, error)
                    .with_rename_target(RenameTarget::Viewport(id))
                    .with_context_menu(move |menu| {
                        menu.event("View", StateEvent::OpenViewport(id));
                        menu.event("Focus", StateEvent::EnterFocusView(id));
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Viewport(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event_if(
                            !is_main_viewport,
                            "Set as Main Viewport",
                            "Already set as main viewport",
                            StateEvent::SetMainViewport(id),
                        );
                        menu.separator();
                        menu.event(
                            "Create New Viewport",
                            StateEvent::CreateResource(ResourceKind::Viewport),
                        );
                    })
                    .build_to(builder);
            }
            builder.close_dir();

            resource_folder(ctx, TreeNodeId::DimensionFolder, "Dimensions")
                .with_count(state.project.dimensions.len())
                .with_context_menu(|menu| {
                    menu.event(
                        "Create New Dimension",
                        StateEvent::CreateResource(ResourceKind::Dimension),
                    );
                })
                .build_to(builder);
            pending_resource_node(ctx, builder, ResourceKind::Dimension);
            for (id, dimension) in state.project.dimensions.list_sorted() {
                let error = state.runtime_project.get_error(id);
                resource_leaf(ctx, TreeNodeId::Dimension(id), dimension, error)
                    .with_rename_target(RenameTarget::Dimension(id))
                    .with_context_menu(move |menu| {
                        menu.event("Inspect", StateEvent::InspectResource(id.into()));
                        menu.rename("Rename", RenameTarget::Dimension(id));
                        menu.event("Delete", StateEvent::DeleteResource(id.into()));
                        menu.separator();
                        menu.event(
                            "Create New Dimension",
                            StateEvent::CreateResource(ResourceKind::Dimension),
                        );
                    })
                    .build_to(builder);
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
