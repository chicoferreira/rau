use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, Deg, LookAt, Pitch, Yaw},
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

const GRID_SIZE: u32 = 256;
const VERTEX_COUNT: u32 = (GRID_SIZE - 1) * (GRID_SIZE - 1) * 6;

pub async fn create_scene(
    _device: &wgpu::Device,
    size: Size2d,
    _file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let shader_id = project.shaders.register(Shader::new(
        "Terrain Shader",
        FilePath::from_str("terrain.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));

    let camera_position = glam::Vec3::new(12.0, 10.0, 12.0);
    let camera_target = glam::Vec3::new(0.0, 2.0, 0.0);
    let view_dir = camera_target - camera_position;

    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(dimension_id));
    camera.set_position(camera_position);
    camera.set_yaw(Yaw::new(Deg(view_dir.z.atan2(view_dir.x).to_degrees())));
    camera.set_pitch(Pitch::new(Deg(view_dir
        .y
        .atan2(view_dir.x.hypot(view_dir.z))
        .to_degrees())));
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

    let heightmap_texture_id = project.textures.register(Texture::new(
        "Heightmap",
        TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        TextureSource::Image(Some(FilePath::from_str("heightmap.png")?)),
    ));
    let heightmap_view_id = project.texture_views.register(TextureView::new(
        "Heightmap View",
        Some(heightmap_texture_id),
        None,
        None,
    ));

    let sampler_id = project.samplers.register(Sampler::new(
        "Linear Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..SamplerSpec::default()
        },
    ));

    let terrain_uniform_id = project.uniforms.register(Uniform::new(
        "Terrain Settings",
        vec![
            UniformField::new(
                "height_scale",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(8.0)),
            ),
            UniformField::new(
                "terrain_size",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(20.0)),
            ),
        ],
    ));

    let terrain_bg_id = project.bind_groups.register(BindGroup::new(
        "Terrain Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(heightmap_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(
                terrain_uniform_id,
            ))),
        ],
    ));

    let color_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let render_view_id = project.texture_views.register(TextureView::new(
        "Viewport Render Target",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let display_view_id = project.texture_views.register(TextureView::new(
        "Viewport Display View",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Linear),
        None,
    ));

    let depth_format = TextureFormat::Depth32Float;
    let depth_texture_id = project.textures.register(Texture::new(
        "Depth Texture",
        depth_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(dimension_id)),
    ));
    let depth_view_id = project.texture_views.register(TextureView::new(
        "Depth Texture View",
        Some(depth_texture_id),
        None,
        None,
    ));

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Terrain Pipeline",
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
            vertices: 0..VERTEX_COUNT,
            instances: 0..1,
        },
        vec![
            BindGroupTarget::Static(camera_bg_id),
            BindGroupTarget::Static(terrain_bg_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Terrain Render Pass",
        RenderPassTarget::new(
            Some(render_view_id),
            LoadOperation::Clear(Color([0.53, 0.72, 0.90, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport",
        Some(display_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
