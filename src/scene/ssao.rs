//! Screen-space ambient occlusion, a deferred port of the LearnOpenGL tutorial
//! <https://learnopengl.com/Advanced-Lighting/SSAO> (code under CC BY-NC 4.0).
//!
//! A backpack sits on the floor of an enclosed room (a cube seen from the inside,
//! normals flipped) lit by one point light, matching the tutorial's `ssao.cpp`.
//! Four passes:
//!
//! 1. **G-buffer**: draws the room and the backpack once into two targets,
//!    view-space position at `@location(0)` and view-space normal at
//!    `@location(1)`.
//! 2. **SSAO**: per pixel, sweeps a hemisphere of samples around the fragment
//!    and counts those hidden behind geometry, into `ssao_raw`.
//! 3. **Blur**: a 4x4 box blur that washes out the sampling noise.
//! 4. **Lighting**: deferred Blinn-Phong, ambient modulated by the occlusion.
//!
//! Two deviations from the tutorial: uniforms have no array type, so the sample
//! kernel and rotation noise are hashed in the shader instead of uploaded; and
//! geometry is shaded with a flat grey albedo, as SSAO is usually shown.

use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, LookAt},
            dimension::Dimension,
            model::Model,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            sampler::{Sampler, SamplerSpec},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{
                Transform, Uniform, UniformField, UniformFieldData, UniformFieldSource,
                camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    utils::wgpu_utils::{PrimitiveState, TextureFormat},
};

/// Vertices in the procedural room cube: 6 faces * 2 triangles * 3 vertices.
const CUBE_VERTICES: u32 = 36;

pub fn build(project: &mut Project) -> AppResult<()> {
    // --- Shaders: room + backpack G-buffer fills, then the full-screen stages. ---
    let room_gbuffer_shader_id = project.shaders.register(Shader::new(
        "Room G-Buffer Shader",
        FilePath::from_str("room_gbuffer.wgsl")?,
    ));
    let backpack_gbuffer_shader_id = project.shaders.register(Shader::new(
        "Backpack G-Buffer Shader",
        FilePath::from_str("backpack_gbuffer.wgsl")?,
    ));
    let ssao_shader_id = project
        .shaders
        .register(Shader::new("SSAO Shader", FilePath::from_str("ssao.wgsl")?));
    let blur_shader_id = project
        .shaders
        .register(Shader::new("Blur Shader", FilePath::from_str("blur.wgsl")?));
    let lighting_shader_id = project.shaders.register(Shader::new(
        "Lighting Shader",
        FilePath::from_str("lighting.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Main Dimension"));

    // --- Camera: starts where the tutorial does, (0, 0, 5) looking down -z, but
    // lifted a touch so the floor occlusion is in frame. ---
    let camera_position = glam::Vec3::new(3.5, 2.0, -3.0);
    let camera_target = glam::Vec3::new(0.0, 0.5, -0.5);
    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.look_at(camera_position, camera_target);
    camera.set_mode(CameraMode::ThirdPerson);
    camera.set_looking_at(LookAt::new(camera_position, camera_target));
    let camera_id = project.cameras.register(camera);

    // SSAO works entirely in view space: the G-buffer stores view-space position
    // and normal, the SSAO pass needs `projection` to reproject samples to the
    // screen, and the lighting pass needs `view` to move the light into view
    // space. All four matrices live in one camera uniform shared by every pass.
    let camera_uniform_id = project.uniforms.register(Uniform::new(
        "Camera",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Position),
            ),
            UniformField::new(
                "view",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::View),
            ),
            UniformField::new(
                "projection",
                UniformFieldSource::new_camera_sourced(Some(camera_id), CameraField::Projection),
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

    let backpack_model_id = project
        .models
        .register(Model::new("Backpack", FilePath::from_str("backpack.obj")?));
    let backpack_transform_uniform_id = project.uniforms.register(Uniform::new(
        "Backpack Transform",
        vec![UniformField::new(
            "model",
            UniformFieldSource::new_transform(Transform {
                position: [0.0, 0.5, 0.0],
                rotation: [-90.0, 0.0, 0.0],
                scale: [1.0; 3],
            }),
        )],
    ));
    let backpack_transform_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Backpack Transform Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(backpack_transform_uniform_id)),
        )],
    ));

    // --- Tunable SSAO parameters (edit live in the inspector). ---
    let ssao_params_uniform_id = project.uniforms.register(Uniform::new(
        "SSAO Params",
        vec![
            UniformField::new(
                "radius",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.5)),
            ),
            UniformField::new(
                "bias",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.025)),
            ),
            UniformField::new(
                "power",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.5)),
            ),
            UniformField::new(
                "kernel_size",
                UniformFieldSource::new_user_defined(UniformFieldData::UInt32(64)),
            ),
        ],
    ));
    let ssao_params_bind_group_id = project.bind_groups.register(BindGroup::new(
        "SSAO Params Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(ssao_params_uniform_id)),
        )],
    ));

    // --- The point light, configurable position and colour, shaded in the
    // deferred lighting pass. Defaults match the tutorial's `lightPos` /
    // `lightColor`. ---
    let light_uniform_id = project.uniforms.register(Uniform::new(
        "Point Light",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([2.0, 4.0, -2.0])),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([0.9, 0.2, 0.2])),
            ),
            UniformField::new(
                "linear",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.09)),
            ),
            UniformField::new(
                "quadratic",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.032)),
            ),
        ],
    ));
    let light_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Point Light Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
    ));

    // Point sampler: the G-buffer holds geometric data (positions/normals), so it
    // must never be linearly blended across edges. SSAO/blur read it likewise.
    let gbuffer_sampler_id = project.samplers.register(Sampler::new(
        "G-Buffer Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerSpec::default()
        },
    ));

    // --- Texture formats. ---
    // Half-float for the G-buffer (view-space position can fall outside [0, 1])
    // and for the single-channel occlusion (no R-only format, so it rides in .r).
    let gbuffer_format = TextureFormat::Rgba16Float;
    let depth_format = TextureFormat::Depth32Float;
    let color_format = TextureFormat::Rgba8UnormSrgb;

    let g_position_texture_id = project.textures.register(Texture::new(
        "G-Buffer Position",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let g_position_view_id = project.texture_views.register(TextureView::new(
        "G-Buffer Position View",
        Some(g_position_texture_id),
        None,
        None,
    ));

    let g_normal_texture_id = project.textures.register(Texture::new(
        "G-Buffer Normal",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let g_normal_view_id = project.texture_views.register(TextureView::new(
        "G-Buffer Normal View",
        Some(g_normal_texture_id),
        None,
        None,
    ));

    // Depth buffer for the G-buffer pass, shared by both attachments.
    let depth_texture_id = project.textures.register(Texture::new(
        "G-Buffer Depth",
        depth_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let depth_view_id = project.texture_views.register(TextureView::new(
        "G-Buffer Depth View",
        Some(depth_texture_id),
        None,
        None,
    ));

    let ssao_raw_texture_id = project.textures.register(Texture::new(
        "SSAO Raw",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let ssao_raw_view_id = project.texture_views.register(TextureView::new(
        "SSAO Raw View",
        Some(ssao_raw_texture_id),
        None,
        None,
    ));

    let ssao_blurred_texture_id = project.textures.register(Texture::new(
        "SSAO Blurred",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let ssao_blurred_view_id = project.texture_views.register(TextureView::new(
        "SSAO Blurred View",
        Some(ssao_blurred_texture_id),
        None,
        None,
    ));

    // --- Final colour target the viewport shows (sRGB render / linear display). ---
    let color_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(dimension_id),
    ));
    let color_render_view_id = project.texture_views.register(TextureView::new(
        "Viewport",
        Some(color_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_id = project.viewports.register(Viewport::new(
        "SSAO Viewport",
        Some(color_render_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    // --- Bind groups that sample earlier passes' outputs. ---
    let gbuffer_sample_bind_group_id = project.bind_groups.register(BindGroup::new(
        "G-Buffer Sample Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(g_position_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(g_normal_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(gbuffer_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
            }),
        ],
    ));

    let blur_sample_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Blur Sample Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(ssao_raw_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(gbuffer_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
            }),
        ],
    ));

    let lighting_sample_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Lighting Sample Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(g_position_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(g_normal_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(ssao_blurred_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(gbuffer_sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::NonFiltering,
            }),
        ],
    ));

    let geometry_primitive = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };
    let fullscreen_primitive = geometry_primitive;

    // --- Geometry pipelines (room + backpack), each filling both G-buffer
    // attachments in one draw. The formats line up with the pass's targets:
    // position at `@location(0)`, normal at `@location(1)`. ---
    let gbuffer_formats = vec![gbuffer_format, gbuffer_format];

    let room_gbuffer_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Room G-Buffer Pipeline",
        geometry_primitive,
        Some(room_gbuffer_shader_id),
        Some(room_gbuffer_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..CUBE_VERTICES,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(camera_bind_group_id)],
        gbuffer_formats.clone(),
        Some(depth_format),
    ));
    let backpack_gbuffer_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Backpack G-Buffer Pipeline",
        geometry_primitive,
        Some(backpack_gbuffer_shader_id),
        Some(backpack_gbuffer_shader_id),
        RenderDrawStrategy::Model {
            model_id: Some(backpack_model_id),
            instances: 0..1,
            mesh_vertex_slot: 0,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(backpack_transform_bind_group_id),
        ],
        gbuffer_formats,
        Some(depth_format),
    ));

    // --- Full-screen pipelines. ---
    let ssao_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "SSAO Pipeline",
        fullscreen_primitive,
        Some(ssao_shader_id),
        Some(ssao_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(gbuffer_sample_bind_group_id),
            BindGroupTarget::Static(ssao_params_bind_group_id),
        ],
        vec![gbuffer_format],
        None,
    ));
    let blur_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "SSAO Blur Pipeline",
        fullscreen_primitive,
        Some(blur_shader_id),
        Some(blur_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(blur_sample_bind_group_id)],
        vec![gbuffer_format],
        None,
    ));
    let lighting_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Lighting Pipeline",
        fullscreen_primitive,
        Some(lighting_shader_id),
        Some(lighting_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
            BindGroupTarget::Static(lighting_sample_bind_group_id),
        ],
        vec![color_format],
        None,
    ));

    // --- Passes, in execution order. ---
    // Position and normal are written together, one target each, by a single
    // draw of the geometry.
    let mut gbuffer_pass = RenderPass::new(
        "G-Buffer Pass",
        vec![
            RenderPassTarget::new(
                Some(g_position_view_id),
                LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
            ),
            RenderPassTarget::new(
                Some(g_normal_view_id),
                LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
            ),
        ],
        Some(RenderPassTarget::new(
            Some(depth_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    gbuffer_pass.set_pipelines(vec![room_gbuffer_pipeline_id, backpack_gbuffer_pipeline_id]);
    let gbuffer_pass_id = project.render_passes.register(gbuffer_pass);

    let mut ssao_pass = RenderPass::new(
        "SSAO Pass",
        vec![RenderPassTarget::new(
            Some(ssao_raw_view_id),
            LoadOperation::Clear(Color([1.0, 1.0, 1.0, 1.0])),
        )],
        None,
    );
    ssao_pass.set_pipelines(vec![ssao_pipeline_id]);
    let ssao_pass_id = project.render_passes.register(ssao_pass);

    let mut blur_pass = RenderPass::new(
        "SSAO Blur Pass",
        vec![RenderPassTarget::new(
            Some(ssao_blurred_view_id),
            LoadOperation::Clear(Color([1.0, 1.0, 1.0, 1.0])),
        )],
        None,
    );
    blur_pass.set_pipelines(vec![blur_pipeline_id]);
    let blur_pass_id = project.render_passes.register(blur_pass);

    let mut lighting_pass = RenderPass::new(
        "Lighting Pass",
        vec![RenderPassTarget::new(
            Some(color_render_view_id),
            LoadOperation::Clear(Color([0.05, 0.05, 0.05, 1.0])),
        )],
        None,
    );
    lighting_pass.set_pipelines(vec![lighting_pipeline_id]);
    let lighting_pass_id = project.render_passes.register(lighting_pass);

    project.presentation.set_render_passes(vec![
        gbuffer_pass_id,
        ssao_pass_id,
        blur_pass_id,
        lighting_pass_id,
    ]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(())
}
