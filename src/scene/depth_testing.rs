//! A depth-buffer visualisation: the same frame shown two ways side by side.
//!
//! An avenue of instanced cubes recedes into the distance in two staggered rows
//! on a checkered floor, with the camera low and looking down the corridor.
//!
//! - The **first pass** draws the floor and the cubes into an sRGB colour target
//!   with a `Depth32Float` depth buffer. The depth buffer is created with
//!   `TEXTURE_BINDING`, so it can be sampled afterwards.
//! - The **second pass** runs a full-screen triangle that samples that depth
//!   buffer, linearises the non-linear `[0, 1]` depth back to eye-space distance,
//!   and writes it as grayscale (near = dark, far = bright).
//!
//! Two viewports share the one camera: the main viewport shows the shaded scene,
//! the second shows the linearised depth. Open both panes to see how depth
//! testing decides what ends up in front.
//!
//! The cubes are pure GPU instancing — one mesh built procedurally in the vertex
//! shader from `vertex_index`, placed on the grid from `instance_index`, no
//! vertex buffers. [`CUBES_PER_ROW`] / [`ROWS`] here must match the constants in
//! `cubes.wgsl`, which derive the instance count of the draw.
//!
//! The linearisation reads `near` / `far` from a uniform; they must match the
//! camera's clip range ([`Z_NEAR`] / [`Z_FAR`]) or the grayscale ramp will be
//! miscalibrated.

use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, ClipRange, Deg, Fov, Pitch, Yaw},
            dimension::Dimension,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{
                Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    ui::size::Size2d,
    utils::{texture_format::TextureFormat, wgpu_utils::PrimitiveState},
};

/// Cubes per row. Must match `cubes.wgsl`.
const CUBES_PER_ROW: u32 = 6;
/// Number of (staggered) rows. Must match `cubes.wgsl`.
const ROWS: u32 = 2;
/// Total instanced cubes drawn per frame.
const CUBE_COUNT: u32 = CUBES_PER_ROW * ROWS;
/// Vertices in the procedural cube: 6 faces * 2 triangles * 3 vertices.
const CUBE_VERTICES: u32 = 36;

/// Camera clip planes. Kept in sync with the `Depth Params` uniform below so the
/// depth-view pass linearises against the same range the projection used.
const Z_NEAR: f32 = 0.1;
const Z_FAR: f32 = 30.0;

pub async fn create_scene(
    _device: &wgpu::Device,
    size: Size2d,
    _file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let cubes_shader_id = project.shaders.register(Shader::new(
        "Cubes Shader",
        FilePath::from_str("cubes.wgsl")?,
    ));
    let floor_shader_id = project.shaders.register(Shader::new(
        "Floor Shader",
        FilePath::from_str("floor.wgsl")?,
    ));
    let depth_view_shader_id = project.shaders.register(Shader::new(
        "Depth View Shader",
        FilePath::from_str("depth_view.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));

    // Low camera looking down the avenue, tilted slightly toward the floor.
    let camera_position = glam::Vec3::new(0.0, 1.6, 5.0);
    let camera_target = glam::Vec3::new(0.0, 0.5, -8.0);
    let view_direction = camera_target - camera_position;

    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_position(camera_position);
    camera.set_fovy(Fov::new(Deg(50.0)));
    camera.set_clip(ClipRange::new(Z_NEAR, Z_FAR));
    camera.set_yaw(Yaw::new(Deg(view_direction
        .z
        .atan2(view_direction.x)
        .to_degrees())));
    camera.set_pitch(Pitch::new(Deg(view_direction
        .y
        .atan2(view_direction.x.hypot(view_direction.z))
        .to_degrees())));
    camera.set_mode(CameraMode::FirstPerson);
    let camera_id = project.cameras.register(camera);

    let camera_uniform_id = project.uniforms.register(Uniform::new(
        "Camera",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Position),
            ),
            UniformField::new(
                "projection_view",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::ProjectionView,
                ),
            ),
        ],
    ));
    let camera_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Camera Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_uniform_id)),
        )],
    ));

    // near/far for linearising the depth buffer. Editable in the inspector to
    // tune the grayscale contrast, but keep them matched to the camera clip range
    // for a physically meaningful ramp.
    let depth_params_uniform_id = project.uniforms.register(Uniform::new(
        "Depth Params",
        vec![
            UniformField::new(
                "near",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(Z_NEAR)),
            ),
            UniformField::new(
                "far",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(Z_FAR)),
            ),
        ],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let depth_format = TextureFormat::Depth32Float;

    // --- Scene colour target (the shaded view) ---
    let color_texture_id = project.textures.register(Texture::new(
        "Scene Colour Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let color_render_view_id = project.texture_views.register(TextureView::new(
        "Scene Colour Render Target",
        Some(color_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let color_display_view_id = project.texture_views.register(TextureView::new(
        "Scene Colour Display View",
        Some(color_texture_id),
        Some(TextureViewFormat::Linear),
        None,
    ));
    let color_viewport_id = project.viewports.register(Viewport::new(
        "Scene Viewport",
        Some(color_display_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    // --- Depth buffer: written in pass 1, sampled in pass 2 ---
    let depth_texture_id = project.textures.register(Texture::new(
        "Depth Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_buffer_view_id = project.texture_views.register(TextureView::new(
        "Depth Buffer View",
        Some(depth_texture_id),
        None,
        None,
    ));

    // --- Depth visualisation target (the grayscale view) ---
    let depth_vis_texture_id = project.textures.register(Texture::new(
        "Depth Vis Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_vis_render_view_id = project.texture_views.register(TextureView::new(
        "Depth Vis Render Target",
        Some(depth_vis_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let depth_vis_display_view_id = project.texture_views.register(TextureView::new(
        "Depth Vis Display View",
        Some(depth_vis_texture_id),
        Some(TextureViewFormat::Linear),
        None,
    ));
    let depth_viewport_id = project.viewports.register(Viewport::new(
        "Depth Viewport",
        Some(depth_vis_display_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    // Group 0 for the depth-view pass: the depth buffer sampled as a depth
    // texture (binding 0, no sampler needed for textureLoad) plus the near/far
    // params (binding 1).
    let depth_sample_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Depth Sample Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(depth_buffer_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Depth,
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(
                depth_params_uniform_id,
            ))),
        ],
    ));

    let scene_primitive = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        // Cubes are drawn double-sided so winding never matters; depth testing
        // sorts the faces regardless.
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };

    // Floor: one quad on the y=0 plane. Camera at group 0.
    let floor_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Floor Pipeline",
        scene_primitive,
        Some(floor_shader_id),
        Some(floor_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..6,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(camera_bind_group_id)],
        color_format,
        Some(depth_format),
    ));

    // Cubes: every cube in one instanced draw, mesh from vertex_index, grid
    // placement from instance_index. Camera at group 0.
    let cubes_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Cubes Pipeline",
        scene_primitive,
        Some(cubes_shader_id),
        Some(cubes_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..CUBE_VERTICES,
            instances: 0..CUBE_COUNT,
        },
        vec![BindGroupTarget::Static(camera_bind_group_id)],
        color_format,
        Some(depth_format),
    ));

    // Depth view: full-screen triangle sampling the depth buffer. No depth
    // buffer of its own. Depth sample bind group at group 0.
    let depth_view_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Depth View Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(depth_view_shader_id),
        Some(depth_view_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(depth_sample_bind_group_id)],
        color_format,
        None,
    ));

    // Pass 1: shade the scene and fill the depth buffer. Floor first, then cubes.
    let mut scene_pass = RenderPass::new(
        "Scene Render Pass",
        RenderPassTarget::new(
            Some(color_render_view_id),
            LoadOperation::Clear(Color([0.52, 0.70, 0.86, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_buffer_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    scene_pass.set_pipelines(vec![floor_pipeline_id, cubes_pipeline_id]);
    let scene_pass_id = project.render_passes.register(scene_pass);

    // Pass 2: sample the depth buffer into the grayscale view.
    let mut depth_view_pass = RenderPass::new(
        "Depth View Render Pass",
        RenderPassTarget::new(
            Some(depth_vis_render_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        ),
        None,
    );
    depth_view_pass.set_pipelines(vec![depth_view_pipeline_id]);
    let depth_view_pass_id = project.render_passes.register(depth_view_pass);

    project
        .presentation
        .set_render_passes(vec![scene_pass_id, depth_view_pass_id]);
    project
        .presentation
        .set_main_viewport(Some(color_viewport_id));
    // The depth viewport shares the camera; open it as a second pane.
    let _ = depth_viewport_id;

    Ok(project)
}
