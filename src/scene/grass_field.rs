//! A field of procedurally generated, wind-swept grass — a pure GPU instancing
//! demo.
//!
//! The whole field is drawn with a single instanced draw call and **no vertex
//! buffers at all**:
//!
//! - `instance_index` selects the blade. Its base position on the grid, height,
//!   orientation, sway phase, and lean are all hashed from that index in the
//!   vertex shader, so every blade is unique without any per-blade CPU data.
//! - `vertex_index` walks a tapered triangle strip up the blade.
//! - A `time` uniform drives a layered wind that scrolls across the field.
//!
//! A second, single-quad pipeline draws the ground underneath so the blades have
//! something to stand on. Both pipelines share the camera bind group and render
//! into the same HDR-ish sRGB target with a depth buffer.
//!
//! The `grass.wgsl` constants (`BLADES_PER_ROW`, `SEGMENTS`) must stay in sync
//! with [`BLADES_PER_ROW`] / [`SEGMENTS`] here — they decide the instance and
//! vertex counts of the draw.

use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, Deg, Pitch, Yaw},
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

/// Blades along one side of the field grid. Must match `grass.wgsl`.
/// 1000 per row -> one million instanced blades.
const BLADES_PER_ROW: u32 = 1000;
/// Total instanced blades drawn per frame.
const BLADE_COUNT: u32 = BLADES_PER_ROW * BLADES_PER_ROW;
/// Height segments per blade. Must match `grass.wgsl`.
const SEGMENTS: u32 = 5;
/// Triangle-strip vertices per blade: two per segment plus the tip.
const VERTS_PER_BLADE: u32 = SEGMENTS * 2 + 1;

pub async fn create_scene(
    _device: &wgpu::Device,
    size: Size2d,
    _file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let grass_shader_id = project.shaders.register(Shader::new(
        "Grass Shader",
        FilePath::from_str("grass.wgsl")?,
    ));
    let ground_shader_id = project.shaders.register(Shader::new(
        "Ground Shader",
        FilePath::from_str("ground.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));

    let camera_position = glam::Vec3::new(0.0, 1.5, 6.5);
    let camera_target = glam::Vec3::new(0.0, 0.5, 0.0);
    let view_direction = camera_target - camera_position;

    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_position(camera_position);
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

    // Wind parameters. `time` drives the sway; the two strengths are editable in
    // the inspector so the wind can be tuned without touching the shader.
    let grass_uniform_id = project.uniforms.register(Uniform::new(
        "Grass",
        vec![
            UniformField::new("time", UniformFieldSource::new_time()),
            UniformField::new(
                "wind_strength",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.35)),
            ),
            UniformField::new(
                "wind_speed",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.2)),
            ),
        ],
    ));
    let grass_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Grass Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(grass_uniform_id)),
        )],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let render_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport Render Target",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let display_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport Display View",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Linear),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "Grass Viewport",
        Some(display_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    let depth_format = TextureFormat::Depth32Float;
    let depth_texture_id = project.textures.register(Texture::new(
        "Depth Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_texture_view_id = project.texture_views.register(TextureView::new(
        "Depth Texture View",
        Some(depth_texture_id),
        None,
        None,
    ));

    // Ground: one quad on the y=0 plane, drawn from the vertex index. Only needs
    // the camera (group 0).
    let ground_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Ground Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(ground_shader_id),
        Some(ground_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..6,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(camera_bind_group_id)],
        color_format,
        Some(depth_format),
    ));

    // Grass: every blade in one instanced draw. Triangle strip per blade, both
    // faces visible (no culling). Camera at group 0, wind at group 1.
    let grass_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Grass Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(grass_shader_id),
        Some(grass_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..VERTS_PER_BLADE,
            instances: 0..BLADE_COUNT,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(grass_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Grass Render Pass",
        RenderPassTarget::new(
            Some(render_texture_view_id),
            LoadOperation::Clear(Color([0.52, 0.70, 0.86, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_texture_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    // Ground first, then grass on top — both share the depth buffer.
    render_pass.set_pipelines(vec![ground_pipeline_id, grass_pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
