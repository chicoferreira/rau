use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, LookAt},
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
    ui::size::Size2d,
    utils::{texture_format::TextureFormat, wgpu_utils::PrimitiveState},
};

const SPHERE_SEGMENTS: u32 = 20;
const SPHERE_RINGS: u32 = 10;
const SPHERE_VERTICES: u32 = SPHERE_SEGMENTS * SPHERE_RINGS * 6;
const GRID_SIZE: u32 = 20;
const SPHERE_COUNT: u32 = GRID_SIZE * GRID_SIZE;

pub async fn create_scene(
    _device: &wgpu::Device,
    size: Size2d,
    _file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let position_shader_id = project.shaders.register(Shader::new(
        "G-Buffer Position Shader",
        FilePath::from_str("gbuffer_position.wgsl")?,
    ));
    let normal_shader_id = project.shaders.register(Shader::new(
        "G-Buffer Normal Shader",
        FilePath::from_str("gbuffer_normal.wgsl")?,
    ));
    let lighting_shader_id = project.shaders.register(Shader::new(
        "Lighting Shader",
        FilePath::from_str("lighting.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));

    let camera_position = glam::Vec3::new(8.0, 10.0, 14.0);
    let camera_target = glam::Vec3::new(0.0, 0.0, 0.0);
    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
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
    let camera_bg_id = project.bind_groups.register(BindGroup::new(
        "Camera Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_uniform_id)),
        )],
    ));

    let scene_uniform_id = project.uniforms.register(Uniform::new(
        "Scene Settings",
        vec![
            UniformField::new("time", UniformFieldSource::new_time()),
            UniformField::new(
                "light_radius",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.8)),
            ),
            UniformField::new(
                "ambient",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.02)),
            ),
        ],
    ));
    let scene_bg_id = project.bind_groups.register(BindGroup::new(
        "Scene Settings Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(scene_uniform_id)),
        )],
    ));

    let gbuffer_sampler_id = project.samplers.register(Sampler::new(
        "G-Buffer Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..SamplerSpec::default()
        },
    ));

    let gbuffer_format = TextureFormat::Rgba16Float;
    let depth_format = TextureFormat::Depth32Float;
    let color_format = TextureFormat::Rgba8UnormSrgb;

    let g_position_tex_id = project.textures.register(Texture::new(
        "G-Buffer Position",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let g_position_view_id = project.texture_views.register(TextureView::new(
        "G-Buffer Position View",
        Some(g_position_tex_id),
        None,
        None,
    ));

    let g_normal_tex_id = project.textures.register(Texture::new(
        "G-Buffer Normal",
        gbuffer_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let g_normal_view_id = project.texture_views.register(TextureView::new(
        "G-Buffer Normal View",
        Some(g_normal_tex_id),
        None,
        None,
    ));

    let depth_tex_id = project.textures.register(Texture::new(
        "Depth",
        depth_format,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_view_id = project.texture_views.register(TextureView::new(
        "Depth View",
        Some(depth_tex_id),
        None,
        None,
    ));

    let viewport_tex_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let viewport_render_view_id = project.texture_views.register(TextureView::new(
        "Viewport Render Target",
        Some(viewport_tex_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_display_view_id = project.texture_views.register(TextureView::new(
        "Viewport Display View",
        Some(viewport_tex_id),
        Some(TextureViewFormat::Linear),
        None,
    ));

    let gbuffer_sample_bg_id = project.bind_groups.register(BindGroup::new(
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

    let geometry_primitive = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };
    let sphere_draw = RenderDrawStrategy::Direct {
        vertices: 0..SPHERE_VERTICES,
        instances: 0..SPHERE_COUNT,
    };
    let fullscreen_draw = RenderDrawStrategy::Direct {
        vertices: 0..3,
        instances: 0..1,
    };

    let position_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Position Pipeline",
        geometry_primitive,
        Some(position_shader_id),
        Some(position_shader_id),
        sphere_draw.clone(),
        vec![BindGroupTarget::Static(camera_bg_id)],
        gbuffer_format,
        Some(depth_format),
    ));
    let normal_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Normal Pipeline",
        geometry_primitive,
        Some(normal_shader_id),
        Some(normal_shader_id),
        sphere_draw,
        vec![BindGroupTarget::Static(camera_bg_id)],
        gbuffer_format,
        Some(depth_format),
    ));
    let lighting_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Lighting Pipeline",
        geometry_primitive,
        Some(lighting_shader_id),
        Some(lighting_shader_id),
        fullscreen_draw,
        vec![
            BindGroupTarget::Static(camera_bg_id),
            BindGroupTarget::Static(scene_bg_id),
            BindGroupTarget::Static(gbuffer_sample_bg_id),
        ],
        color_format,
        None,
    ));

    let mut position_pass = RenderPass::new(
        "G-Buffer Position Pass",
        RenderPassTarget::new(
            Some(g_position_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    position_pass.set_pipelines(vec![position_pipeline_id]);
    let position_pass_id = project.render_passes.register(position_pass);

    let mut normal_pass = RenderPass::new(
        "G-Buffer Normal Pass",
        RenderPassTarget::new(
            Some(g_normal_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    normal_pass.set_pipelines(vec![normal_pipeline_id]);
    let normal_pass_id = project.render_passes.register(normal_pass);

    let mut lighting_pass = RenderPass::new(
        "Lighting Pass",
        RenderPassTarget::new(
            Some(viewport_render_view_id),
            LoadOperation::Clear(Color([0.01, 0.01, 0.015, 1.0])),
        ),
        None,
    );
    lighting_pass.set_pipelines(vec![lighting_pipeline_id]);
    let lighting_pass_id = project.render_passes.register(lighting_pass);

    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport",
        Some(viewport_display_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    project.presentation.set_render_passes(vec![
        position_pass_id,
        normal_pass_id,
        lighting_pass_id,
    ]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
