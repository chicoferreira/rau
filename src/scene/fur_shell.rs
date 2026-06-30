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
            model::{Model, ModelRuntime},
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

const SHELL_COUNT: u32 = 48;

pub async fn create_scene(device: &wgpu::Device, file_storage: &FileStorage) -> AppResult<Project> {
    let mut project = Project::default();

    let shader_id = project
        .shaders
        .register(Shader::new("Fur Shader", FilePath::from_str("fur.wgsl")?));

    let dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Main Dimension"));

    let camera_position = glam::Vec3::new(-0.2, 1.2, 2.1);
    let camera_target = glam::Vec3::new(-0.3, 0.7, 0.2);

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

    let light_uniform_id = project.uniforms.register(Uniform::new(
        "Light",
        vec![
            UniformField::new(
                "position",
                UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([3.0, 4.0, 2.0])),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 0.95, 0.85])),
            ),
        ],
    ));
    let light_bg_id = project.bind_groups.register(BindGroup::new(
        "Light Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
    ));

    let fur_uniform_id = project.uniforms.register(Uniform::new(
        "Fur Settings",
        vec![
            UniformField::new(
                "fur_length",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.06)),
            ),
            UniformField::new(
                "density",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1000.0)),
            ),
            UniformField::new("time", UniformFieldSource::new_time()),
        ],
    ));
    let fur_bg_id = project.bind_groups.register(BindGroup::new(
        "Fur Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(fur_uniform_id)),
        )],
    ));

    let bunny_source = FilePath::from_str("bunny.obj")?;
    let bunny_model = Model::new("Bunny", bunny_source.clone());
    let (_, _) = ModelRuntime::load_from_obj_file(
        bunny_source,
        file_storage,
        bunny_model.vertex_buffer_spec().clone(),
        device.clone(),
    )
    .await?;
    let bunny_model_id = project.models.register(bunny_model);

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
        "Viewport",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
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
        "Fur Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(shader_id),
        Some(shader_id),
        RenderDrawStrategy::Model {
            model_id: Some(bunny_model_id),
            instances: 0..SHELL_COUNT,
            mesh_vertex_slot: 0,
        },
        vec![
            BindGroupTarget::Static(camera_bg_id),
            BindGroupTarget::Static(light_bg_id),
            BindGroupTarget::Static(fur_bg_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Fur Render Pass",
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
        Some(render_view_id),
        Some(dimension_id),
        Some(camera_id),
    ));

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
