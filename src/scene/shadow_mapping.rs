//! A minimal shadow-mapping demo: a handful of boxes on a floor, lit by a single
//! spot light that casts hard shadows.
//!
//! Shadow mapping is a two-pass technique:
//!
//! - The **shadow pass** renders the scene geometry from the *light's* point of
//!   view into a depth texture — the shadow map. Nothing is shaded; only the
//!   depth of the nearest surface to the light is kept. The light is modelled as
//!   a second [`Camera`], so its `ProjectionView` matrix (the light-space
//!   transform) comes straight from the camera uniform machinery. The pass still
//!   needs a colour attachment it never reads, so it renders into a throwaway
//!   colour texture.
//! - The **scene pass** renders from the real camera. For each fragment it
//!   reprojects the world position into light space, looks the stored depth up in
//!   the shadow map, and compares: if the fragment is further from the light than
//!   whatever the light saw first, it is in shadow. A small slope-scaled bias
//!   fights acne and a 3x3 PCF tap softens the edges.
//!
//! All geometry is procedural — one unit cube built in the vertex shader from
//! `vertex_index`, placed and scaled per `instance_index` (instance 0 is the
//! flattened floor box, the rest are the shadow casters). [`OBJECT_COUNT`] here
//! must match the object table in both `scene.wgsl` and `shadow.wgsl`, which
//! derive the instance count of the draw and the transforms.

use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, ClipRange, Deg, Fov, LookAt},
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

/// Objects drawn each frame: instance 0 is the floor, the rest are cubes. Must
/// match the object table in `scene.wgsl` and `shadow.wgsl`.
const OBJECT_COUNT: u32 = 5;
/// Vertices in the procedural cube: 6 faces * 2 triangles * 3 vertices.
const CUBE_VERTICES: u32 = 36;
/// Side length of the (square) shadow map: a fixed 4K resolution, independent of
/// the light viewport's on-screen size. Higher = crisper shadow edges.
const SHADOW_MAP_SIZE: u32 = 4096;

pub async fn create_scene(
    _device: &wgpu::Device,
    size: Size2d,
    _file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let scene_shader_id = project.shaders.register(Shader::new(
        "Scene Shader",
        FilePath::from_str("scene.wgsl")?,
    ));
    let shadow_shader_id = project.shaders.register(Shader::new(
        "Shadow Shader",
        FilePath::from_str("shadow.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));
    // Fixed square dimension for the shadow map and light camera: aspect stays
    // 1:1 and the resolution never follows the light viewport's pane size (the
    // viewport owns no dimension, so resizing it can't write back here).
    let shadow_dimension_id = project.dimensions.register(Dimension::new(
        "Shadow Map Dimension",
        Size2d::new(SHADOW_MAP_SIZE, SHADOW_MAP_SIZE),
    ));

    // --- The viewing camera: a third-person orbit around the cluster. ---
    let camera_position = glam::Vec3::new(2.0, 2.5, 5.0);
    let camera_target = glam::Vec3::new(0.0, 0.6, 0.0);
    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.look_at(camera_position, camera_target);
    camera.set_mode(CameraMode::FirstPerson);
    camera.set_looking_at(LookAt::new(camera_position, camera_target));
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

    // --- The light, modelled as a spot-light camera looking down at the scene.
    // Its projection-view matrix is the light-space transform used both to render
    // the shadow map and to reproject fragments when sampling it. ---
    let light_position = glam::Vec3::new(7.0, 5.5, 2.0);
    let light_target = glam::Vec3::new(0.0, 0.0, 0.0);
    let mut light_camera = Camera::new("Light".to_string());
    light_camera.set_dimension_id(Some(shadow_dimension_id));
    light_camera.set_fovy(Fov::new(Deg(70.0)));
    light_camera.set_clip(ClipRange::new(1.0, 40.0));
    light_camera.look_at(light_position, light_target);
    let light_camera_id = project.cameras.register(light_camera);

    let light_uniform_id = project.uniforms.register(Uniform::new(
        "Light",
        vec![
            UniformField::new(
                "projection_view",
                UniformFieldSource::new_camera_sourced(
                    Some(light_camera_id),
                    CameraField::ProjectionView,
                ),
            ),
            UniformField::new(
                "position",
                UniformFieldSource::new_camera_sourced(
                    Some(light_camera_id),
                    CameraField::Position,
                ),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 0.95, 0.85])),
            ),
        ],
    ));
    let light_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Light Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let depth_format = TextureFormat::Depth32Float;

    // --- Scene colour target (what the viewport shows). ---
    let color_texture_id = project.textures.register(Texture::new(
        "Scene Colour Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let color_render_view_id = project.texture_views.register(TextureView::new(
        "Scene Colour",
        Some(color_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "Scene Viewport",
        Some(color_render_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    // --- Scene depth buffer (camera-view depth testing). ---
    let scene_depth_texture_id = project.textures.register(Texture::new(
        "Scene Depth Texture",
        depth_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let scene_depth_view_id = project.texture_views.register(TextureView::new(
        "Scene Depth View",
        Some(scene_depth_texture_id),
        None,
        None,
    ));

    // --- The shadow map: depth written in the shadow pass, sampled in the scene
    // pass. TEXTURE_BINDING lets the scene shader read it back. ---
    let shadow_map_texture_id = project.textures.register(Texture::new(
        "Shadow Map Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(shadow_dimension_id)),
    ));
    let shadow_map_view_id = project.texture_views.register(TextureView::new(
        "Shadow Map View",
        Some(shadow_map_texture_id),
        None,
        None,
    ));

    // The shadow pass needs a colour attachment alongside the depth it actually
    // cares about. Rather than throw it away, the shadow shader shades the scene
    // from the light's point of view so the colour target doubles as a "what the
    // light sees" image, shown in a second viewport that drives the light camera.
    let light_view_texture_id = project.textures.register(Texture::new(
        "Light View Colour Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(shadow_dimension_id)),
    ));
    let light_view_render_view_id = project.texture_views.register(TextureView::new(
        "Light View",
        Some(light_view_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    // Open this pane and focus it to fly the light around; the shadows in the
    // main viewport follow it live. It deliberately owns no dimension (None), so
    // resizing or moving the pane never rewrites the fixed 4K shadow dimension —
    // the light camera's aspect and the shadow map resolution stay put, and the
    // 4K render is simply scaled to fit the pane.
    let light_viewport_id = project.viewports.register(Viewport::new(
        "Light Viewport",
        Some(light_view_render_view_id),
        None,
        Some(light_camera_id),
    ));

    // Group 2 of the scene pass: the shadow map sampled as a depth texture
    // (textureLoad needs no sampler).
    let shadow_sample_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Shadow Sample Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Texture {
                texture_view_id: Some(shadow_map_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Depth,
            },
        )],
    ));

    // Boxes are drawn double-sided so the procedural cube winding never matters;
    // depth testing sorts the faces regardless.
    let primitive = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };

    // Shadow pass pipeline: geometry transformed by the light matrix (group 0).
    // Its depth buffer becomes the shadow map; its colour is the light's view.
    let shadow_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Shadow Pipeline",
        primitive,
        Some(shadow_shader_id),
        Some(shadow_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..CUBE_VERTICES,
            instances: 0..OBJECT_COUNT,
        },
        vec![BindGroupTarget::Static(light_bind_group_id)],
        color_format,
        Some(depth_format),
    ));

    // Scene pass pipeline: camera (group 0), light (group 1), shadow map (group 2).
    let scene_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Scene Pipeline",
        primitive,
        Some(scene_shader_id),
        Some(scene_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..CUBE_VERTICES,
            instances: 0..OBJECT_COUNT,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
            BindGroupTarget::Static(shadow_sample_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    // Pass 1: render the scene from the light. The depth buffer becomes the
    // shadow map; the colour target feeds the light viewport.
    let mut shadow_pass = RenderPass::new(
        "Shadow Pass",
        RenderPassTarget::new(
            Some(light_view_render_view_id),
            LoadOperation::Clear(Color([0.16, 0.28, 0.42, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(shadow_map_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    shadow_pass.set_pipelines(vec![shadow_pipeline_id]);
    let shadow_pass_id = project.render_passes.register(shadow_pass);

    // Pass 2: shade the scene from the camera, sampling the shadow map.
    let mut scene_pass = RenderPass::new(
        "Scene Pass",
        RenderPassTarget::new(
            Some(color_render_view_id),
            LoadOperation::Clear(Color([0.16, 0.28, 0.42, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(scene_depth_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    scene_pass.set_pipelines(vec![scene_pipeline_id]);
    let scene_pass_id = project.render_passes.register(scene_pass);

    project
        .presentation
        .set_render_passes(vec![shadow_pass_id, scene_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));
    // The light viewport shares no schedule of its own; it just displays the
    // shadow pass's colour target and drives the light camera when focused.
    let _ = light_viewport_id;

    Ok(project)
}
