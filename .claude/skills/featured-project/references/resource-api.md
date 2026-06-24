# rau scene resource API cookbook

A `create_scene` function builds a `Project` by registering resources into typed
storages. Each `register(...)` call returns an opaque `Id` you pass to other
resources to wire them together. Build order matters only in that an id must
exist before it's referenced — there are no implicit dependencies otherwise.

The canonical examples to copy from live in `src/scene/`:

- `game_of_life.rs` — compute-driven, ping-pong storage textures, full-screen
  triangle display. No external assets, shaders only.
- `model.rs` — single OBJ model with diffuse/normal/specular maps, a camera, a
  point light, and a depth buffer. The smallest "real 3D" scene.
- `full_example.rs` — everything: HDR pipeline, equirect→cubemap compute,
  skybox, instanced models, multiple render passes.

When in doubt, read the closest example end-to-end and adapt it. The notes below
are a map, not a replacement for reading the source.

## The skeleton every scene shares

```rust
pub async fn create_scene(
    device: &wgpu::Device,        // name it `_device` if unused (e.g. shader-only scenes)
    size: Size2d,                  // the generation resolution, currently 1080x1080
    file_storage: &FileStorage,    // name it `_file_storage` if you load no models
) -> AppResult<Project> {
    let mut project = Project::default();
    // ... register resources ...
    project.presentation.set_render_passes(vec![/* in execution order */]);
    project.presentation.set_compute_passes(vec![/* in execution order */]);
    project.presentation.set_main_viewport(Some(viewport_id));
    Ok(project)
}
```

`set_compute_passes` is only needed when the scene has compute work. The
presentation's `render_passes` / `compute_passes` vectors define explicit
execution order — list them in the order they must run each frame.

## The storages on `Project`

Each is a `Storage<T>` with a `.register(value) -> Id` method:

`shaders`, `viewports`, `uniforms`, `bind_groups`, `textures`, `texture_views`,
`samplers`, `dimensions`, `cameras`, `models`, `render_pipelines`,
`render_passes`, `compute_passes`.

## Binding convention (shaders must agree with the scene)

This is the most common source of bugs. WGSL `@group`/`@binding` indices are
**positional**, derived from how you wire the scene — they are not written
anywhere explicitly:

- **`@group(N)`** = the position of the bind group in the pipeline's
  `BindGroupTarget` vector (for render pipelines) or in the `ComputePass`'s
  bind-group vector (for compute). First entry is group 0.
- **`@binding(M)`** = the position of the entry inside that `BindGroup`'s
  `entries` vector. First entry is binding 0.

`BindGroupTarget::ModelMaterial` occupies a group slot too — it expands to the
material's textures+sampler bindings for the bound model, in the standard
diffuse/normal/(specular) layout. See the material WGSL in an existing model
project's `shader.wgsl` for the exact binding numbers it expects.

## Resources by example

### Shader

```rust
let shader_id = project.shaders.register(
    Shader::new("Main Shader", FilePath::from_str("shader.wgsl")?),
);
```

The `FilePath` is relative to the project folder. A scene with several stages
registers one `Shader` per `.wgsl` file (see `game_of_life.rs`: init / simulate /
copy / render).

### Dimension

A named resolution. Resources size themselves from a dimension so they resize
together.

```rust
let dim_id = project.dimensions.register(Dimension::new("Main Dimension", size));
// A fixed-size grid independent of the viewport:
let grid_dim_id = project.dimensions.register(
    Dimension::new("Grid Dimension", Size2d::new(160, 90)),
);
```

### Texture + TextureView

```rust
let tex_id = project.textures.register(Texture::new(
    "Viewport Texture",
    TextureFormat::Rgba8UnormSrgb,                 // see utils::texture_format::TextureFormat
    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    TextureSource::Dimension(Some(dim_id)),        // sized from a Dimension
));
let view_id = project.texture_views.register(TextureView::new(
    "Viewport View",
    Some(tex_id),
    None,   // Option<TextureViewFormat>: Some(Srgb)/Some(Linear) to reinterpret, None = inherit
    None,   // Option<wgpu::TextureViewDimension>: Cube, D2Array, etc.; None = D2
));
```

`TextureSource` variants seen in the codebase:
- `TextureSource::Dimension(Some(dim_id))` — sized from a dimension (most common).
- `TextureSource::Image(Some(path))` — loaded from an image file (e.g. an HDR).
  See `full_example::create_texture`.
- `TextureSource::Manual { size: wgpu::Extent3d { .. } }` — explicit size, e.g.
  a cubemap with `depth_or_array_layers: 6`.

Storage textures (written by compute) need
`wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING`.
The viewport's color target needs `RENDER_ATTACHMENT`; a depth buffer is just a
`Depth32Float` texture with `RENDER_ATTACHMENT`.

For a viewport that goes through an sRGB color target, register two views over
the same texture: one `TextureViewFormat::Srgb` as the render target, one
`TextureViewFormat::Linear` for display (see `model.rs`).

### Sampler

```rust
let sampler_id = project.samplers.register(Sampler::new(
    "Material Sampler",
    SamplerSpec {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..SamplerSpec::default()
    },
));
```

### Uniform

A uniform is a named struct of fields. Each field has a source:

```rust
let u_id = project.uniforms.register(Uniform::new("Point Light", vec![
    UniformField::new("position",
        UniformFieldSource::new_user_defined(UniformFieldData::Vec3f([2.5, 3.2, 2.5]))),
    UniformField::new("color",
        UniformFieldSource::new_user_defined(UniformFieldData::Rgb([1.0, 0.82, 0.68]))),
]));
```

`UniformFieldSource` constructors:
- `new_user_defined(UniformFieldData)` — an editable value in the inspector.
- `new_camera_sourced(Some(camera_id), CameraField::X)` — driven by a camera.
  Fields: `Position`, `View`, `ProjectionView`, `InverseProjection`,
  `InverseView` (see `uniform::camera::CameraField`).
- `new_time()` — seconds since start, an `f32`.

`UniformFieldData` variants and their WGSL types:
`UInt32` → `u32`, `Float` → `f32`, `Vec2f` → `vec2<f32>`, `Vec3f`/`Rgb` →
`vec3<f32>`, `Vec4f`/`Rgba` → `vec4<f32>`, `Mat4x4f` → `mat4x4<f32>`. `Rgb`/
`Rgba` render as a color picker; the plain vectors as number fields.

### BindGroup + BindGroupEntry

Each entry declares its visibility and the resource it binds. Visibility
constructors: `BindGroupEntry::new_compute(...)`,
`BindGroupEntry::new_vertex_fragment(...)`.

```rust
let bg_id = project.bind_groups.register(BindGroup::new("Camera Bind Group", vec![
    BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(u_id))),
]));
```

`BindGroupResource` variants:
- `Uniform(Some(uniform_id))`
- `Texture { texture_view_id, view_dimension, sample_type }` — a sampled texture.
- `StorageTexture { texture_view_id, view_dimension, access }` — a storage texture
  (`wgpu::StorageTextureAccess::WriteOnly` etc.).
- `Sampler { sampler_id, sampler_binding_type }`

### Camera

```rust
let mut camera = Camera::new("Camera".to_string());
camera.set_dimension_id(Some(dim_id));     // for aspect ratio
camera.set_position(glam::Vec3::new(2.0, 1.3, 2.5));
camera.set_yaw(Yaw::new(Deg(angle_degrees)));
camera.set_pitch(Pitch::new(Deg(angle_degrees)));
camera.set_mode(CameraMode::ThirdPerson);  // optional; default is first-person
camera.set_looking_at(LookAt::new(position, target)); // third-person orbit target
let camera_id = project.cameras.register(camera);
```

### Model (OBJ)

Loading the OBJ at generation time is required to build the material bind groups.
The `.obj`, `.mtl`, and texture files must already be in the project folder
before the generate command runs.

```rust
let model_source = FilePath::from_str("metal_barrel/metal_barrel.obj")?;
let mut model = Model::new("Metal Barrel", model_source.clone());
let (model_runtime, _) = ModelRuntime::load_from_obj_file(
    model_source, file_storage, model.vertex_buffer_spec().clone(), device.clone(),
).await?;
let material_bind_group_ids = MaterialBindGroupsConfig {
    textures: vec![
        (TextureType::Diffuse, default_texture_format(TextureType::Diffuse)),
        (TextureType::Normal,  default_texture_format(TextureType::Normal)),
        (TextureType::Specular, TextureFormat::Rgba8Unorm),
    ],
    sampler: SamplerSetting::Existing(material_sampler_id),
}.create_bind_groups(&mut project, model_runtime.materials(), model.label())?;
model.set_material_bind_group_ids(material_bind_group_ids);
let model_id = project.models.register(model);
```

### RenderPipeline

```rust
let pipeline_id = project.render_pipelines.register(RenderPipeline::new(
    "Model Pipeline",
    PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back), // None to disable culling (full-screen tris)
        polygon_mode: wgpu::PolygonMode::Fill,
    },
    Some(vertex_shader_id),
    Some(fragment_shader_id),               // same id is fine if one file has both entry points
    RenderDrawStrategy::Model {             // or ::Direct { vertices, instances }
        model_id: Some(model_id),
        instances: 0..1,
        mesh_vertex_slot: 0,
    },
    vec![                                   // group order == @group index in the shader
        BindGroupTarget::ModelMaterial,
        BindGroupTarget::Static(camera_bind_group_id),
        BindGroupTarget::Static(light_bind_group_id),
    ],
    color_format,                           // must match the render pass target's texture format
    Some(depth_format),                     // None if no depth buffer
));
```

`RenderDrawStrategy::Direct { vertices: 0..3, instances: 0..1 }` is the
full-screen-triangle / procedural-geometry path (no vertex buffer).

### RenderPass

```rust
let mut render_pass = RenderPass::new(
    "Model Render Pass",
    RenderPassTarget::new(Some(color_view_id), LoadOperation::Clear(Color([r,g,b,a]))),
    Some(RenderPassTarget::new(Some(depth_view_id), LoadOperation::Clear(1.0))), // depth, or None
);
render_pass.set_pipelines(vec![pipeline_id]); // pipelines run in this order within the pass
let render_pass_id = project.render_passes.register(render_pass);
```

`LoadOperation::default()` is "load existing contents"; `Clear(..)` resets first.

### ComputePass

```rust
let pass_id = project.compute_passes.register(ComputePass::new(
    "Simulate",
    vec![bind_group_id],                 // group order == @group index in the compute shader
    Some(shader_id),
    WorkGroups::new(x, y, z),            // dispatch counts; use n.div_ceil(workgroup_size)
    DispatchPolicy::Periodic { interval: Duration::from_millis(200) },
));
```

`DispatchPolicy` variants: `OnChange` (run once / when inputs change, e.g. an init
or a one-shot precompute) and `Periodic { interval }` (steady cadence,
framerate-independent). `WorkGroups::new(a, b, c)` must cover the data size given
the shader's `@workgroup_size`.

### Viewport

The on-screen output. The main viewport is what the app displays and what the
thumbnail is captured from.

```rust
let viewport_id = project.viewports.register(Viewport::new(
    "Viewport",
    Some(display_texture_view_id),
    Some(dim_id),
    Some(camera_id),   // None for 2D / full-screen-shader scenes with no camera
));
project.presentation.set_main_viewport(Some(viewport_id));
```
</content>
</invoke>
