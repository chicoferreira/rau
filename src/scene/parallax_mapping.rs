//! Parallax occlusion mapping on a single brick quad.
//!
//! A port of the LearnOpenGL "Parallax Mapping" chapter
//! (<https://learnopengl.com/Advanced-Lighting/Parallax-Mapping>) using the same
//! brick textures: a diffuse map, a tangent-space normal map, and a displacement
//! (depth) map.
//!
//! There is no model — the quad is six vertices built procedurally in the vertex
//! shader via [`RenderDrawStrategy::Direct`], with a fixed tangent frame
//! (T = +X, B = -Y, N = +Z). The vertex shader transforms the camera, light, and
//! fragment positions into tangent space; the fragment shader marches the view
//! ray through the depth map (parallax occlusion mapping) to offset the texture
//! coordinates before sampling the diffuse and normal maps and applying
//! Blinn-Phong lighting.
//!
//! The `height_scale` uniform controls how pronounced the displacement is — bump
//! it up in the inspector to exaggerate the effect, or down to 0 to fall back to
//! plain normal mapping.

use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, Deg, Fov, LookAt},
            dimension::Dimension,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            sampler::{Sampler, SamplerSpec},
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

    let shader_id = project.shaders.register(Shader::new(
        "Parallax Shader",
        FilePath::from_str("shader.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Main Dimension"));
    let camera_position = glam::Vec3::new(-1.7, 0.0, 3.0);
    let camera_target = glam::Vec3::new(0.0, 0.0, 0.0);

    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_fovy(Fov::new(Deg(45.0)));
    camera.look_at(camera_position, camera_target);
    camera.set_mode(CameraMode::ThirdPerson);
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

    let light_uniform_id = project.uniforms.register(Uniform::new(
        "Point Light",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([0.5, 0.6, 1.2])),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 0.95, 0.85])),
            ),
        ],
    ));
    let light_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Point Light Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
    ));

    // The strength of the parallax displacement. 0.1 matches the LearnOpenGL
    // chapter; editable in the inspector.
    let parallax_uniform_id = project.uniforms.register(Uniform::new(
        "Parallax",
        vec![UniformField::new(
            "height_scale",
            UniformFieldSource::new_user_defined(UniformFieldData::Float(0.1)),
        )],
    ));
    let parallax_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Parallax Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(parallax_uniform_id)),
        )],
    ));

    // The three brick maps. Diffuse is colour data (sRGB); the normal and
    // displacement maps are raw data (linear).
    let material_sampler_id = project.samplers.register(Sampler::new(
        "Material Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..SamplerSpec::default()
        },
    ));

    let diffuse_texture_id = project.textures.register(image_texture(
        "Brick Diffuse",
        FilePath::from_str("bricks/bricks2.jpg")?,
        TextureFormat::Rgba8UnormSrgb,
    ));
    let normal_texture_id = project.textures.register(image_texture(
        "Brick Normal",
        FilePath::from_str("bricks/bricks2_normal.jpg")?,
        TextureFormat::Rgba8Unorm,
    ));
    let depth_texture_id = project.textures.register(image_texture(
        "Brick Displacement",
        FilePath::from_str("bricks/bricks2_disp.jpg")?,
        TextureFormat::Rgba8Unorm,
    ));

    let diffuse_view_id = project.texture_views.register(TextureView::new(
        "Brick Diffuse View",
        Some(diffuse_texture_id),
        None,
        None,
    ));
    let normal_view_id = project.texture_views.register(TextureView::new(
        "Brick Normal View",
        Some(normal_texture_id),
        None,
        None,
    ));
    let depth_view_id = project.texture_views.register(TextureView::new(
        "Brick Displacement View",
        Some(depth_texture_id),
        None,
        None,
    ));

    let sampled = |view_id| BindGroupResource::Texture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
    };

    let diffuse_resource = sampled(diffuse_view_id);
    let normal_resource = sampled(normal_view_id);
    let depth_resource = sampled(depth_view_id);
    let material_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Brick Material Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(diffuse_resource),
            BindGroupEntry::new_vertex_fragment(normal_resource),
            BindGroupEntry::new_vertex_fragment(depth_resource),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(material_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let render_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "Parallax Viewport",
        Some(render_texture_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    let depth_format = TextureFormat::Depth32Float;
    let depth_buffer_texture_id = project.textures.register(Texture::new(
        "Depth Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_buffer_view_id = project.texture_views.register(TextureView::new(
        "Depth Texture View",
        Some(depth_buffer_texture_id),
        None,
        None,
    ));

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Parallax Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            // The quad is double-sided so it stays visible as the camera orbits.
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(shader_id),
        Some(shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..6,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(material_bind_group_id),
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
            BindGroupTarget::Static(parallax_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Parallax Render Pass",
        RenderPassTarget::new(
            Some(render_texture_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_buffer_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}

fn image_texture(label: &str, path: FilePath, format: TextureFormat) -> Texture {
    Texture::new(
        label,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        TextureSource::Image(Some(path)),
    )
}
