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
            model::{Model, ModelRuntime, TextureType},
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
    utils::{
        derive::default_texture_format,
        derive_modal_material::{MaterialBindGroupsConfig, SamplerSetting},
        texture_format::TextureFormat,
        wgpu_utils::PrimitiveState,
    },
};

pub async fn create_scene(
    device: &wgpu::Device,
    size: Size2d,
    file_storage: &FileStorage,
) -> AppResult<Project> {
    let mut project = Project::default();

    let shader_id = project.shaders.register(Shader::new(
        "Main Shader",
        FilePath::from_str("shader.wgsl")?,
    ));

    let dimension_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));

    let camera_position = glam::Vec3::new(2.0, 1.3, 2.5);
    let camera_target = glam::Vec3::new(0.0, 0.9, 0.0);

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
                UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([2.5, 3.2, 2.5])),
            ),
            UniformField::new(
                "color",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 0.82, 0.68])),
            ),
        ],
    ));
    let light_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Point Light Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(light_uniform_id)),
        )],
    ));

    let material_sampler_id = project.samplers.register(Sampler::new(
        "Material Sampler",
        SamplerSpec {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..SamplerSpec::default()
        },
    ));

    let model_source = FilePath::from_str("metal_barrel/metal_barrel.obj")?;
    let mut model = Model::new("Metal Barrel", model_source.clone());
    let (model_runtime, _) = ModelRuntime::load_from_obj_file(
        model_source,
        file_storage,
        model.vertex_buffer_spec().clone(),
        device.clone(),
    )
    .await?;
    let material_bind_group_ids = MaterialBindGroupsConfig {
        textures: vec![
            (
                TextureType::Diffuse,
                default_texture_format(TextureType::Diffuse),
            ),
            (
                TextureType::Normal,
                default_texture_format(TextureType::Normal),
            ),
            (TextureType::Specular, TextureFormat::Rgba8Unorm),
        ],
        sampler: SamplerSetting::Existing(material_sampler_id),
    }
    .create_bind_groups(&mut project, model_runtime.materials(), model.label())?;
    model.set_material_bind_group_ids(material_bind_group_ids);
    let model_id = project.models.register(model);

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
        "Model Viewport",
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

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Model Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(shader_id),
        Some(shader_id),
        RenderDrawStrategy::Model {
            model_id: Some(model_id),
            instances: 0..1,
            mesh_vertex_slot: 0,
        },
        vec![
            BindGroupTarget::ModelMaterial,
            BindGroupTarget::Static(camera_bind_group_id),
            BindGroupTarget::Static(light_bind_group_id),
        ],
        color_format,
        Some(depth_format),
    ));

    let mut render_pass = RenderPass::new(
        "Model Render Pass",
        RenderPassTarget::new(
            Some(render_texture_view_id),
            LoadOperation::Clear(Color([0.018, 0.025, 0.045, 1.0])),
        ),
        Some(RenderPassTarget::new(
            Some(depth_texture_view_id),
            LoadOperation::Clear(1.0),
        )),
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    project.presentation.set_render_passes(vec![render_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
