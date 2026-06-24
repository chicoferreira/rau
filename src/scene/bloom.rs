use crate::{
    error::AppResult,
    file::file_storage::FileStorage,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            dimension::Dimension,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            sampler::{Sampler, SamplerSpec},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::{TextureView, TextureViewFormat},
            uniform::{Uniform, UniformField, UniformFieldData, UniformFieldSource},
            viewport::Viewport,
        },
    },
    ui::size::Size2d,
    utils::{texture_format::TextureFormat, wgpu_utils::PrimitiveState},
};

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
    let extract_shader_id = project.shaders.register(Shader::new(
        "Extract Shader",
        FilePath::from_str("extract.wgsl")?,
    ));
    let blur_h_shader_id = project.shaders.register(Shader::new(
        "Blur Horizontal Shader",
        FilePath::from_str("blur_horizontal.wgsl")?,
    ));
    let blur_v_shader_id = project.shaders.register(Shader::new(
        "Blur Vertical Shader",
        FilePath::from_str("blur_vertical.wgsl")?,
    ));
    let composite_shader_id = project.shaders.register(Shader::new(
        "Composite Shader",
        FilePath::from_str("composite.wgsl")?,
    ));

    let main_dim_id = project
        .dimensions
        .register(Dimension::new("Main Dimension", size));
    let bloom_dim_id = project.dimensions.register(Dimension::new(
        "Bloom Dimension",
        Size2d::new(size.width() / 2, size.height() / 2),
    ));

    let hdr_format = TextureFormat::Rgba16Float;
    let hdr_texture_id = project.textures.register(Texture::new(
        "HDR Texture",
        hdr_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(main_dim_id)),
    ));
    let hdr_view_id = project.texture_views.register(TextureView::new(
        "HDR View",
        Some(hdr_texture_id),
        None,
        None,
    ));

    let bloom_a_texture_id = project.textures.register(Texture::new(
        "Bloom A Texture",
        hdr_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(bloom_dim_id)),
    ));
    let bloom_a_view_id = project.texture_views.register(TextureView::new(
        "Bloom A View",
        Some(bloom_a_texture_id),
        None,
        None,
    ));

    let bloom_b_texture_id = project.textures.register(Texture::new(
        "Bloom B Texture",
        hdr_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::Dimension(Some(bloom_dim_id)),
    ));
    let bloom_b_view_id = project.texture_views.register(TextureView::new(
        "Bloom B View",
        Some(bloom_b_texture_id),
        None,
        None,
    ));

    let viewport_format = TextureFormat::Rgba8UnormSrgb;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        viewport_format,
        wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC,
        TextureSource::Dimension(Some(main_dim_id)),
    ));
    let viewport_srgb_view_id = project.texture_views.register(TextureView::new(
        "Viewport Srgb View",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Srgb),
        None,
    ));
    let viewport_linear_view_id = project.texture_views.register(TextureView::new(
        "Viewport Linear View",
        Some(viewport_texture_id),
        Some(TextureViewFormat::Linear),
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

    let time_uniform_id = project.uniforms.register(Uniform::new(
        "Time",
        vec![UniformField::new("time", UniformFieldSource::new_time())],
    ));

    let bloom_uniform_id = project.uniforms.register(Uniform::new(
        "Bloom Settings",
        vec![
            UniformField::new(
                "threshold",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.0)),
            ),
            UniformField::new(
                "intensity",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.5)),
            ),
        ],
    ));

    let scene_bg_id = project.bind_groups.register(BindGroup::new(
        "Scene Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(time_uniform_id)),
        )],
    ));

    let extract_bg_id = project.bind_groups.register(BindGroup::new(
        "Extract Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(hdr_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(bloom_uniform_id))),
        ],
    ));

    let blur_h_bg_id = project.bind_groups.register(BindGroup::new(
        "Blur H Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(bloom_a_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    ));

    let blur_v_bg_id = project.bind_groups.register(BindGroup::new(
        "Blur V Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(bloom_b_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    ));

    let composite_bg_id = project.bind_groups.register(BindGroup::new(
        "Composite Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(hdr_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(bloom_a_view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(bloom_uniform_id))),
        ],
    ));

    let fullscreen_tri = PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: None,
        polygon_mode: wgpu::PolygonMode::Fill,
    };
    let direct_draw = RenderDrawStrategy::Direct {
        vertices: 0..3,
        instances: 0..1,
    };

    let scene_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Scene Pipeline",
        fullscreen_tri,
        Some(scene_shader_id),
        Some(scene_shader_id),
        direct_draw.clone(),
        vec![BindGroupTarget::Static(scene_bg_id)],
        hdr_format,
        None,
    ));

    let extract_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Extract Pipeline",
        fullscreen_tri,
        Some(extract_shader_id),
        Some(extract_shader_id),
        direct_draw.clone(),
        vec![BindGroupTarget::Static(extract_bg_id)],
        hdr_format,
        None,
    ));

    let blur_h_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Blur H Pipeline",
        fullscreen_tri,
        Some(blur_h_shader_id),
        Some(blur_h_shader_id),
        direct_draw.clone(),
        vec![BindGroupTarget::Static(blur_h_bg_id)],
        hdr_format,
        None,
    ));

    let blur_v_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Blur V Pipeline",
        fullscreen_tri,
        Some(blur_v_shader_id),
        Some(blur_v_shader_id),
        direct_draw.clone(),
        vec![BindGroupTarget::Static(blur_v_bg_id)],
        hdr_format,
        None,
    ));

    let composite_pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Composite Pipeline",
        fullscreen_tri,
        Some(composite_shader_id),
        Some(composite_shader_id),
        direct_draw,
        vec![BindGroupTarget::Static(composite_bg_id)],
        viewport_format,
        None,
    ));

    let mut scene_pass = RenderPass::new(
        "Scene Pass",
        RenderPassTarget::new(
            Some(hdr_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        ),
        None,
    );
    scene_pass.set_pipelines(vec![scene_pipeline_id]);
    let scene_pass_id = project.render_passes.register(scene_pass);

    let mut extract_pass = RenderPass::new(
        "Extract Pass",
        RenderPassTarget::new(
            Some(bloom_a_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 0.0])),
        ),
        None,
    );
    extract_pass.set_pipelines(vec![extract_pipeline_id]);
    let extract_pass_id = project.render_passes.register(extract_pass);

    let mut blur_h_pass = RenderPass::new(
        "Blur Horizontal Pass",
        RenderPassTarget::new(Some(bloom_b_view_id), LoadOperation::default()),
        None,
    );
    blur_h_pass.set_pipelines(vec![blur_h_pipeline_id]);
    let blur_h_pass_id = project.render_passes.register(blur_h_pass);

    let mut blur_v_pass = RenderPass::new(
        "Blur Vertical Pass",
        RenderPassTarget::new(Some(bloom_a_view_id), LoadOperation::default()),
        None,
    );
    blur_v_pass.set_pipelines(vec![blur_v_pipeline_id]);
    let blur_v_pass_id = project.render_passes.register(blur_v_pass);

    let mut blur_h2_pass = RenderPass::new(
        "Blur Horizontal Pass 2",
        RenderPassTarget::new(Some(bloom_b_view_id), LoadOperation::default()),
        None,
    );
    blur_h2_pass.set_pipelines(vec![blur_h_pipeline_id]);
    let blur_h2_pass_id = project.render_passes.register(blur_h2_pass);

    let mut blur_v2_pass = RenderPass::new(
        "Blur Vertical Pass 2",
        RenderPassTarget::new(Some(bloom_a_view_id), LoadOperation::default()),
        None,
    );
    blur_v2_pass.set_pipelines(vec![blur_v_pipeline_id]);
    let blur_v2_pass_id = project.render_passes.register(blur_v2_pass);

    let mut composite_pass = RenderPass::new(
        "Composite Pass",
        RenderPassTarget::new(Some(viewport_srgb_view_id), LoadOperation::default()),
        None,
    );
    composite_pass.set_pipelines(vec![composite_pipeline_id]);
    let composite_pass_id = project.render_passes.register(composite_pass);

    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport",
        Some(viewport_linear_view_id),
        Some(main_dim_id),
        None,
    ));

    project.presentation.set_render_passes(vec![
        scene_pass_id,
        extract_pass_id,
        blur_h_pass_id,
        blur_v_pass_id,
        blur_h2_pass_id,
        blur_v2_pass_id,
        composite_pass_id,
    ]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
