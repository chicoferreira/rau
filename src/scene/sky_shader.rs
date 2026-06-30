//! A procedural Preetham sky drawn on a single full-screen triangle.
//!
//! Ported from Embark Studios' rust-gpu `sky-shader` example (dual MIT/
//! Apache-2.0): <https://github.com/EmbarkStudios/rust-gpu>.
//!
//! There is no geometry and no depth buffer: the vertex shader emits an
//! oversized triangle that covers the screen, and the fragment shader
//! reconstructs a per-pixel view ray from the camera's inverse projection and
//! inverse view matrices, then evaluates the sky in that direction. The sun
//! position is a user-editable uniform.
//!
//! Because the ray comes from the bound camera, orbiting the viewport sweeps the
//! sky just like a real one.

use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::Camera,
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
    utils::{texture_format::TextureFormat, wgpu_utils::PrimitiveState},
};

pub async fn create_scene() -> AppResult<Project> {
    let mut project = Project::default();

    let shader_id = project
        .shaders
        .register(Shader::new("Sky Shader", FilePath::from_str("sky.wgsl")?));

    let dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Main Dimension"));

    // All camera fields are defaults; only the label and dimension differ.
    let mut camera = Camera::new("Main Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    let camera_id = project.cameras.register(camera);

    // The sky shader reconstructs the view ray from the inverse projection and
    // inverse view, so the camera feeds five matrices into one uniform.
    let camera_uniform_id = project.uniforms.register(Uniform::new(
        "Camera Buffer",
        vec![
            UniformField::new(
                "view_position",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Position),
            ),
            UniformField::new(
                "view",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::View),
            ),
            UniformField::new(
                "proj_view",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::ProjectionView,
                ),
            ),
            UniformField::new(
                "inv_proj",
                UniformFieldSource::new_camera_sourced(
                    Some(camera_id),
                    CameraField::InverseProjection,
                ),
            ),
            UniformField::new(
                "inv_view",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::InverseView),
            ),
        ],
    ));
    let camera_bind_group_id = project.bind_groups.register(BindGroup::new(
        "camera bind group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_uniform_id)),
        )],
    ));

    let sky_uniform_id = project.uniforms.register(Uniform::new(
        "Sky",
        vec![UniformField::new(
            "sun_position",
            UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([10.0, 0.5, 0.0])),
        )],
    ));
    let sky_bind_group_id = project.bind_groups.register(BindGroup::new(
        "sky bind group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(sky_uniform_id)),
        )],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let viewport_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport Texture View",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport Texture",
        Some(viewport_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "sky pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(shader_id),
        Some(shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(sky_bind_group_id),
        ],
        color_format,
        None,
    ));

    let mut render_pass = RenderPass::new(
        "Sky Render Pass",
        RenderPassTarget::new(
            Some(viewport_texture_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        ),
        None,
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
