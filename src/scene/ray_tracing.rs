//! The final scene of *Ray Tracing in One Weekend*, path traced on the GPU.
//!
//! Ported from Peter Shirley, Trevor David Black and Steve Hollasch's
//! *Ray Tracing in One Weekend* (CC0): <https://raytracing.github.io>.
//!
//! The scene is rendered progressively with accumulation buffers that
//! get reset when the scene inputs change (camera, scene properties, etc).
//!
//! Three stages make that work:
//!
//! 1. `Generate Scene` bakes the spheres into a texture. It is set to run
//!    `OnChange`, so it only re-runs when the scene uniform changes.
//! 2. `Reset Accumulation` clears the accumulation buffers. It is also
//!    `OnChange` and it binds every uniform that affects the image, including
//!    the camera, so that the policy fires whenever one of them changes.
//! 3. `Trace` traces one frame's worth of samples per pixel and folds them into
//!    the accumulation buffers.
//!
//! There are four `R32Float` accumulation buffers in play here. Three for the RGB
//! channels and one for the count. It would be neater if there were only a
//! `Rgba32Float` texture (with the alpha being the count), but WebGPU only
//! guarantees read-write storage access for the single-channel 32-bit formats,
//! so a copy shader would be needed instead. This way, for each texel, the trace
//! happens and the value is written directly. It also doesn't need any barriers,
//! because one invocation only touches its own texel.
//!
//! Finally a render pass reassembles the three channels onto a full-screen
//! triangle, applying the book's gamma correction (a plain square root).
//!
//! The scene data lives in a `Rgba32Float` texture three rows tall, one column
//! per sphere, laid out as a structure of arrays:
//!
//! | row | rgba                                              |
//! |-----|---------------------------------------------------|
//! | 0   | `center.xyz`, `radius` (0 marks an empty slot)    |
//! | 1   | `albedo.rgb`, fuzz (metal) or index of refraction |
//! | 2   | material kind, unused                             |
//!
//! It would be neater as well to use storage buffers instead of storage textures
//! for this, but Rau still doesn't support them.
use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            camera::{Camera, CameraMode, Deg, Pitch, Yaw},
            compute_pass::{ComputePass, DispatchPolicy, DispatchSize, DispatchUnit, WorkSize},
            dimension::{Axis, Dimension, DimensionRef},
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::TextureView,
            uniform::{
                Uniform, UniformField, UniformFieldData, UniformFieldSource, camera::CameraField,
            },
            viewport::Viewport,
        },
    },
    ui::size::Size2d,
    utils::wgpu_utils::{PrimitiveState, TextureFormat},
};

const SCENE_CAPACITY: u32 = 512;

pub async fn create_scene() -> AppResult<Project> {
    let mut project = Project::default();

    let generate_shader_id = project.shaders.register(Shader::new(
        "Generate Shader",
        FilePath::from_str("generate.wgsl")?,
    ));
    let reset_shader_id = project.shaders.register(Shader::new(
        "Reset Shader",
        FilePath::from_str("reset.wgsl")?,
    ));
    let trace_shader_id = project.shaders.register(Shader::new(
        "Trace Shader",
        FilePath::from_str("trace.wgsl")?,
    ));
    let display_shader_id = project.shaders.register(Shader::new(
        "Display Shader",
        FilePath::from_str("display.wgsl")?,
    ));

    let display_dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Display Dimension"));

    let scene_dimension_id = project.dimensions.register(Dimension::new_persistent(
        "Scene Capacity",
        Size2d::new(SCENE_CAPACITY, 3), // check the table in this file's header to see the reason for this 3
    ));

    let mut camera = Camera::new("Camera".to_string());
    camera.set_dimension_id(Some(display_dimension_id));
    camera.set_position(glam::Vec3::new(7.0, 1.5, 2.5));
    camera.set_yaw(Yaw::new(Deg(-160.0)));
    camera.set_pitch(Pitch::new(Deg(-10.0)));
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

    let scene_uniform_id = project.uniforms.register(Uniform::new(
        "Scene Settings",
        vec![
            UniformField::new(
                "scene_size",
                UniformFieldSource::new_dimension(Some(scene_dimension_id)),
            ),
            UniformField::new(
                "seed",
                UniformFieldSource::new_user_defined(UniformFieldData::UInt32(0x9E37_79B9)),
            ),
            UniformField::new(
                "grid_extent",
                UniformFieldSource::new_user_defined(UniformFieldData::UInt32(11)),
            ),
            UniformField::new(
                "metal_chance",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.15)),
            ),
            UniformField::new(
                "glass_chance",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.05)),
            ),
            UniformField::new(
                "small_radius",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.2)),
            ),
            UniformField::new(
                "glass_ior",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(1.5)),
            ),
        ],
    ));

    let render_uniform_id = project.uniforms.register(Uniform::new(
        "Render Settings",
        vec![
            UniformField::new(
                "sky_zenith",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([0.5, 0.7, 1.0])),
            ),
            UniformField::new(
                "max_bounces",
                UniformFieldSource::new_user_defined(UniformFieldData::UInt32(8)),
            ),
            UniformField::new(
                "sky_horizon",
                UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 1.0, 1.0])),
            ),
            UniformField::new(
                "samples_per_frame",
                UniformFieldSource::new_user_defined(UniformFieldData::UInt32(1)),
            ),
            UniformField::new(
                "defocus_angle",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(0.0)), // disabled at the start
            ),
            UniformField::new(
                "focus_distance",
                UniformFieldSource::new_user_defined(UniformFieldData::Float(10.0)),
            ),
        ],
    ));

    let storage_usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
    let sampled = |view_id| BindGroupResource::Texture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        sample_type: wgpu::TextureSampleType::Float { filterable: false },
    };
    let storage = |view_id, access| BindGroupResource::StorageTexture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        access,
    };

    let scene_texture_id = project.textures.register(Texture::new(
        "Scene",
        TextureFormat::Rgba32Float,
        storage_usage,
        TextureSource::dimension(scene_dimension_id),
    ));
    let scene_view_id = project.texture_views.register(TextureView::new(
        "Scene View",
        Some(scene_texture_id),
        None,
        None,
    ));

    let mut accumulation = |label| {
        let texture_id = project.textures.register(Texture::new(
            label,
            TextureFormat::R32Float,
            storage_usage,
            TextureSource::dimension(display_dimension_id),
        ));
        project.texture_views.register(TextureView::new(
            format!("{label} View"),
            Some(texture_id),
            None,
            None,
        ))
    };
    let red_view_id = accumulation("Accumulation Red");
    let green_view_id = accumulation("Accumulation Green");
    let blue_view_id = accumulation("Accumulation Blue");
    let sample_count_view_id = accumulation("Sample Count");

    let generate_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Generate Bind Group",
        vec![
            BindGroupEntry::new_compute(storage(
                scene_view_id,
                wgpu::StorageTextureAccess::WriteOnly,
            )),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(scene_uniform_id))),
        ],
    ));

    let reset_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Reset Bind Group",
        vec![
            BindGroupEntry::new_compute(storage(
                red_view_id,
                wgpu::StorageTextureAccess::WriteOnly,
            )),
            BindGroupEntry::new_compute(storage(
                green_view_id,
                wgpu::StorageTextureAccess::WriteOnly,
            )),
            BindGroupEntry::new_compute(storage(
                blue_view_id,
                wgpu::StorageTextureAccess::WriteOnly,
            )),
            BindGroupEntry::new_compute(storage(
                sample_count_view_id,
                wgpu::StorageTextureAccess::WriteOnly,
            )),
            // These entries are not read in the shader. They are only present here so the
            // `OnChange` policy re-runs the shader whenever one of them changes.
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(camera_uniform_id))),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(render_uniform_id))),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(scene_uniform_id))),
        ],
    ));

    let trace_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Trace Bind Group",
        vec![
            BindGroupEntry::new_compute(storage(
                red_view_id,
                wgpu::StorageTextureAccess::ReadWrite, // the reason for the separate textures
            )),
            BindGroupEntry::new_compute(storage(
                green_view_id,
                wgpu::StorageTextureAccess::ReadWrite,
            )),
            BindGroupEntry::new_compute(storage(
                blue_view_id,
                wgpu::StorageTextureAccess::ReadWrite,
            )),
            BindGroupEntry::new_compute(storage(
                sample_count_view_id,
                wgpu::StorageTextureAccess::ReadWrite,
            )),
            BindGroupEntry::new_compute(sampled(scene_view_id)),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(camera_uniform_id))),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(render_uniform_id))),
        ],
    ));

    // One invocation per sphere slot: the scene texture's width is the capacity,
    // and its three rows are written together, so only the x axis is dispatched.
    let scene_dispatch = DispatchSize {
        x: WorkSize::Dimension(DimensionRef {
            id: Some(scene_dimension_id),
            axis: Axis::Width,
        }),
        y: WorkSize::Fixed(1),
        z: WorkSize::Fixed(1),
        unit: DispatchUnit::Invocation {
            workgroup_size: [64, 1, 1],
        },
    };
    // One invocation per pixel for everything that walks the framebuffer.
    let screen_dispatch = DispatchSize::new_dimension(
        display_dimension_id,
        1,
        DispatchUnit::Invocation {
            workgroup_size: [8, 8, 1],
        },
    );

    let generate_pass_id = project.compute_passes.register(ComputePass::new(
        "Generate Scene",
        vec![generate_bind_group_id],
        Some(generate_shader_id),
        scene_dispatch,
        DispatchPolicy::OnChange,
    ));
    let reset_pass_id = project.compute_passes.register(ComputePass::new(
        "Reset Accumulation",
        vec![reset_bind_group_id],
        Some(reset_shader_id),
        screen_dispatch,
        DispatchPolicy::OnChange,
    ));
    let trace_pass_id = project.compute_passes.register(ComputePass::new(
        "Trace",
        vec![trace_bind_group_id],
        Some(trace_shader_id),
        screen_dispatch,
        DispatchPolicy::EveryFrame,
    ));

    let color_format = TextureFormat::Rgba8Unorm;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(display_dimension_id),
    ));
    let viewport_view_id = project.texture_views.register(TextureView::new(
        "Viewport",
        Some(viewport_texture_id),
        None,
        None,
    ));

    let display_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Display Bind Group",
        vec![
            BindGroupEntry::new_vertex_fragment(sampled(red_view_id)),
            BindGroupEntry::new_vertex_fragment(sampled(green_view_id)),
            BindGroupEntry::new_vertex_fragment(sampled(blue_view_id)),
        ],
    ));

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Display Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(display_shader_id),
        Some(display_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(display_bind_group_id)],
        color_format,
        None,
    ));

    let mut render_pass = RenderPass::new(
        "Display Render Pass",
        RenderPassTarget::new(
            Some(viewport_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        ),
        None,
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport",
        Some(viewport_view_id),
        Some(display_dimension_id),
        Some(camera_id),
    ));

    project.presentation.set_render_passes(vec![render_pass_id]);
    project
        .presentation
        .set_compute_passes(vec![generate_pass_id, reset_pass_id, trace_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(project)
}
