//! Conway's Game of Life running entirely on the GPU.
//!
//! Two `Rgba8Unorm` storage textures (grid A and grid B) are ping-ponged each
//! simulation step:
//!
//! 1. `Init` seeds a pseudo-random soup into grid A. It is `OnChange`, so it runs
//!    exactly once.
//! 2. `Simulate` reads grid A and computes one generation into grid B.
//! 3. `Copy` writes grid B back into grid A.
//! 4. A render pass samples grid A onto a full-screen triangle for display.
//!
//! `Simulate` and `Copy` are `Periodic` at [`STEP_INTERVAL`], so the simulation
//! advances at a steady rate independent of the framerate.

use crate::{
    error::AppResult,
    project::{
        Project,
        paths::FilePath,
        resource::{
            bindgroup::{BindGroup, BindGroupEntry, BindGroupResource},
            compute_pass::{ComputePass, DispatchPolicy, DispatchSize, DispatchUnit},
            dimension::Dimension,
            render_pass::{Color, LoadOperation, RenderPass, RenderPassTarget},
            render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
            shader::Shader,
            texture::{Texture, TextureSource},
            texture_view::TextureView,
            uniform::{Uniform, UniformField, UniformFieldData, UniformFieldSource},
            viewport::Viewport,
        },
    },
    ui::size::Size2d,
    utils::wgpu_utils::{PrimitiveState, TextureFormat},
};

pub fn build(project: &mut Project) -> AppResult<()> {
    // Shaders: one per compute stage plus the display shader.
    let init_shader_id = project
        .shaders
        .register(Shader::new("Init Shader", FilePath::from_str("init.wgsl")?));
    let simulate_shader_id = project.shaders.register(Shader::new(
        "Simulate Shader",
        FilePath::from_str("simulate.wgsl")?,
    ));
    let copy_shader_id = project
        .shaders
        .register(Shader::new("Copy Shader", FilePath::from_str("copy.wgsl")?));
    let render_shader_id = project.shaders.register(Shader::new(
        "Render Shader",
        FilePath::from_str("render.wgsl")?,
    ));

    // The grid runs at a fixed resolution, independent of the (resizable) viewport.
    let grid_dimension_id = project.dimensions.register(Dimension::new_persistent(
        "Grid Dimension",
        Size2d::new(160, 90),
    ));
    let display_dimension_id = project
        .dimensions
        .register(Dimension::new_runtime("Display Dimension"));

    // Two ping-pong state textures. Each is written as a storage texture and read
    // back as a sampled texture, so both usages are required.
    let grid_usage = wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING;
    let grid_a_texture_id = project.textures.register(Texture::new(
        "Grid A",
        TextureFormat::Rgba8Unorm,
        grid_usage,
        TextureSource::dimension(grid_dimension_id),
    ));
    let grid_b_texture_id = project.textures.register(Texture::new(
        "Grid B",
        TextureFormat::Rgba8Unorm,
        grid_usage,
        TextureSource::dimension(grid_dimension_id),
    ));
    let grid_a_view_id = project.texture_views.register(TextureView::new(
        "Grid A View",
        Some(grid_a_texture_id),
        None,
        None,
    ));
    let grid_b_view_id = project.texture_views.register(TextureView::new(
        "Grid B View",
        Some(grid_b_texture_id),
        None,
        None,
    ));

    // Helpers for the two flavours of grid binding used by the compute passes.
    let sampled_grid = |view_id| BindGroupResource::Texture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        sample_type: wgpu::TextureSampleType::Float { filterable: true },
    };
    let storage_grid = |view_id| BindGroupResource::StorageTexture {
        texture_view_id: Some(view_id),
        view_dimension: wgpu::TextureViewDimension::D2,
        access: wgpu::StorageTextureAccess::WriteOnly,
    };

    // The seed that drives the init shader's hash. A plain `u32` uniform, so
    // changing it in the inspector reshuffles the starting soup.
    let seed_uniform_id = project.uniforms.register(Uniform::new(
        "Seed",
        vec![UniformField::new(
            "seed",
            UniformFieldSource::new_user_defined(UniformFieldData::UInt32(0x9E37_79B9)),
        )],
    ));

    // Init: write the seed into grid A, using the seed uniform for the hash.
    let init_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Init Bind Group",
        vec![
            BindGroupEntry::new_compute(storage_grid(grid_a_view_id)),
            BindGroupEntry::new_compute(BindGroupResource::Uniform(Some(seed_uniform_id))),
        ],
    ));

    // Simulate: read grid A, write the next generation into grid B.
    let simulate_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Simulate Bind Group",
        vec![
            BindGroupEntry::new_compute(sampled_grid(grid_a_view_id)),
            BindGroupEntry::new_compute(storage_grid(grid_b_view_id)),
        ],
    ));

    // Copy: read grid B, write it back into grid A.
    let copy_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Copy Bind Group",
        vec![
            BindGroupEntry::new_compute(sampled_grid(grid_b_view_id)),
            BindGroupEntry::new_compute(storage_grid(grid_a_view_id)),
        ],
    ));

    let grid_dispatch = DispatchSize::new_dimension(
        grid_dimension_id,
        1,
        DispatchUnit::Invocation {
            workgroup_size: [8, 8, 1],
        },
    );
    // Init seeds the grid once; Simulate and Copy advance one generation per
    // `STEP_INTERVAL`. Giving both the same interval keeps them in lockstep, so a
    // step is always a Simulate (A -> B) immediately followed by a Copy (B -> A).
    let init_pass_id = project.compute_passes.register(ComputePass::new(
        "Init",
        vec![init_bind_group_id],
        Some(init_shader_id),
        grid_dispatch,
        DispatchPolicy::OnChange,
    ));

    /// How often the simulation advances one generation: two steps per second.
    const STEP_INTERVAL: instant::Duration = instant::Duration::from_millis(200);

    let simulate_pass_id = project.compute_passes.register(ComputePass::new(
        "Simulate",
        vec![simulate_bind_group_id],
        Some(simulate_shader_id),
        grid_dispatch,
        DispatchPolicy::Periodic {
            interval: STEP_INTERVAL,
        },
    ));
    let copy_pass_id = project.compute_passes.register(ComputePass::new(
        "Copy",
        vec![copy_bind_group_id],
        Some(copy_shader_id),
        grid_dispatch,
        DispatchPolicy::Periodic {
            interval: STEP_INTERVAL,
        },
    ));

    // Display: a full-screen triangle that samples grid A.
    let color_format = TextureFormat::Rgba8Unorm;
    let viewport_texture_id = project.textures.register(Texture::new(
        "Viewport Texture",
        color_format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        TextureSource::dimension(display_dimension_id),
    ));
    let viewport_texture_view_id = project.texture_views.register(TextureView::new(
        "Viewport",
        Some(viewport_texture_id),
        None,
        None,
    ));

    let render_bind_group_id = project.bind_groups.register(BindGroup::new(
        "Render Bind Group",
        vec![BindGroupEntry::new_vertex_fragment(sampled_grid(
            grid_a_view_id,
        ))],
    ));

    let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
        "Render Pipeline",
        PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
        },
        Some(render_shader_id),
        Some(render_shader_id),
        RenderDrawStrategy::Direct {
            vertices: 0..3,
            instances: 0..1,
        },
        vec![BindGroupTarget::Static(render_bind_group_id)],
        vec![color_format],
        None,
    ));

    let mut render_pass = RenderPass::new(
        "Render Pass",
        vec![RenderPassTarget::new(
            Some(viewport_texture_view_id),
            LoadOperation::Clear(Color([0.0, 0.0, 0.0, 1.0])),
        )],
        None,
    );
    render_pass.set_pipelines(vec![pipeline_id]);
    let render_pass_id = project.render_passes.register(render_pass);

    let viewport_id = project.viewports.register(Viewport::new(
        "Viewport",
        Some(viewport_texture_view_id),
        Some(display_dimension_id),
        None,
    ));

    project.presentation.set_render_passes(vec![render_pass_id]);
    // Execution order: seed, then step (simulate then commit) each frame they fire.
    project
        .presentation
        .set_compute_passes(vec![init_pass_id, simulate_pass_id, copy_pass_id]);
    project.presentation.set_main_viewport(Some(viewport_id));

    Ok(())
}
