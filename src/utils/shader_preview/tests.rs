use super::*;
use crate::project::Creatable;
use crate::project::resource::bindgroup::{BindGroup, BindGroupEntry, BindGroupResource};
use crate::project::resource::model::Model;
use crate::project::resource::render_pipeline::{
    BindGroupTarget, RenderDrawStrategy, RenderPipeline,
};
use crate::project::resource::sampler::Sampler;
use crate::project::resource::texture::Texture;
use crate::project::resource::texture_view::TextureView;
use crate::project::resource::uniform::{
    Uniform, UniformField, UniformFieldData, UniformFieldSource,
};
use crate::project::storage::Storage;
use crate::utils::texture_format::TextureFormat;

#[derive(Default)]
struct TestStores {
    bind_groups: Storage<BindGroup>,
    uniforms: Storage<Uniform>,
    texture_views: Storage<TextureView>,
    samplers: Storage<Sampler>,
    textures: Storage<Texture>,
    models: Storage<Model>,
}

impl TestStores {
    fn ctx(&self) -> ShaderGenCtx<'_> {
        ShaderGenCtx {
            bind_groups: &self.bind_groups,
            uniforms: &self.uniforms,
            texture_views: &self.texture_views,
            samplers: &self.samplers,
            textures: &self.textures,
            models: &self.models,
        }
    }
}

fn user_field(label: &str, data: UniformFieldData) -> UniformField {
    UniformField::new(label, UniformFieldSource::new_user_defined(data))
}

fn wgsl(item: &impl ShaderInterface, ctx: &ShaderGenCtx) -> String {
    render(item, ctx, Language::Wgsl)
}

fn glsl(item: &impl ShaderInterface, ctx: &ShaderGenCtx) -> String {
    render(item, ctx, Language::Glsl)
}

/// Wraps a generated GLSL snippet into a minimal shader and checks that
/// naga's GLSL frontend (the one shaders are compiled with) accepts it.
#[track_caller]
fn assert_glsl_parses(snippet: &str, stage: naga::ShaderStage) {
    let shader = format!("#version 450\n{snippet}\nvoid main() {{}}");
    if let Err(e) = naga::front::glsl::Frontend::default()
        .parse(&naga::front::glsl::Options::from(stage), &shader)
    {
        let error = e.emit_to_string(&shader);
        panic!("generated GLSL should parse:\n{error}");
    }
}

#[test]
fn uniform_struct_matches_issue_example() {
    let uniform = Uniform::new(
        "Light",
        vec![
            user_field("position", UniformFieldData::Vec3f([0.0; 3])),
            user_field("color", UniformFieldData::Rgb([1.0; 3])),
        ],
    );

    let expected = "struct Light {\n    position: vec3<f32>,\n    color: vec3<f32>,\n}";
    assert_eq!(wgsl(&uniform, &TestStores::default().ctx()), expected);

    let expected = "struct Light {\n    vec3 position;\n    vec3 color;\n};";
    assert_eq!(glsl(&uniform, &TestStores::default().ctx()), expected);
}

#[test]
fn bind_group_emits_struct_then_declarations() {
    let mut stores = TestStores::default();
    let camera_id = stores.uniforms.register(Uniform::new(
        "Camera",
        vec![user_field(
            "view_proj",
            UniformFieldData::Mat4x4f([[0.0; 4]; 4]),
        )],
    ));
    let view_id = stores
        .texture_views
        .register(TextureView::new("Albedo View", None, None, None));
    let sampler_id = stores.samplers.create("Linear Sampler".to_string());

    let bind_group = BindGroup::new(
        "Material",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Uniform(Some(camera_id))),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    );

    let item = BindGroupAt::new(0, &bind_group);

    let expected = "struct Camera {\n    view_proj: mat4x4<f32>,\n}\n\n\
        @group(0) @binding(0) var<uniform> camera: Camera;\n\
        @group(0) @binding(1) var albedo_view: texture_2d<f32>;\n\
        @group(0) @binding(2) var linear_sampler: sampler;";

    assert_eq!(wgsl(&item, &stores.ctx()), expected);

    // The uniform struct is inlined into the uniform block in GLSL.
    let expected = "layout(set = 0, binding = 0) uniform Camera {\n\
        \u{20}   mat4 view_proj;\n\
        } camera;\n\
        layout(set = 0, binding = 1) uniform texture2D albedo_view;\n\
        layout(set = 0, binding = 2) uniform sampler linear_sampler;";

    assert_eq!(glsl(&item, &stores.ctx()), expected);
    assert_glsl_parses(expected, naga::ShaderStage::Fragment);
}

#[test]
fn bind_group_with_unknown_group_renders_underscore() {
    let mut stores = TestStores::default();
    let camera_id = stores.uniforms.register(Uniform::new(
        "Camera",
        vec![user_field(
            "view_proj",
            UniformFieldData::Mat4x4f([[0.0; 4]; 4]),
        )],
    ));
    let bind_group = BindGroup::new(
        "Camera BG",
        vec![BindGroupEntry::new_vertex_fragment(
            BindGroupResource::Uniform(Some(camera_id)),
        )],
    );

    let item = BindGroupAt::new(None, &bind_group);

    let expected = "struct Camera {\n    view_proj: mat4x4<f32>,\n}\n\n\
        @group(_) @binding(0) var<uniform> camera: Camera;";

    assert_eq!(wgsl(&item, &stores.ctx()), expected);

    let expected = "layout(set = _, binding = 0) uniform Camera {\n\
        \u{20}   mat4 view_proj;\n\
        } camera;";

    assert_eq!(glsl(&item, &stores.ctx()), expected);
}

#[test]
fn pipeline_model_material_slot_derives_first_material_bind_group() {
    let mut stores = TestStores::default();
    let view_id = stores
        .texture_views
        .register(TextureView::new("Diffuse View", None, None, None));
    let sampler_id = stores.samplers.create("Material Sampler".to_string());
    let material_bg_id = stores.bind_groups.register(BindGroup::new(
        "Material BG",
        vec![
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Texture {
                texture_view_id: Some(view_id),
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
            }),
            BindGroupEntry::new_vertex_fragment(BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                sampler_binding_type: wgpu::SamplerBindingType::Filtering,
            }),
        ],
    ));

    let mut model = Model::create("Sphere".to_string());
    model.set_material_bind_group_ids(vec![None, Some(material_bg_id)]);
    let model_id = stores.models.register(model);

    let pipeline = RenderPipeline::new(
        "Pipeline",
        wgpu::PrimitiveState::default(),
        None,
        None,
        RenderDrawStrategy::Model {
            model_id: Some(model_id),
            instances: 0..1,
            mesh_vertex_slot: 0,
        },
        vec![BindGroupTarget::ModelMaterial],
        TextureFormat::Rgba8Unorm,
        None,
    );

    let rendered = wgsl(&pipeline, &stores.ctx());
    assert!(
        rendered.contains("// group/set 0 is bound to each mesh's material bind group"),
        "expected material comment in:\n{rendered}"
    );
    assert!(
        rendered.contains("@group(0) @binding(0) var diffuse_view: texture_2d<f32>;"),
        "expected derived texture binding in:\n{rendered}"
    );
    assert!(
        rendered.contains("@group(0) @binding(1) var material_sampler: sampler;"),
        "expected derived sampler binding in:\n{rendered}"
    );
}

#[test]
fn model_emits_vertex_input_struct_with_locations() {
    use crate::project::Creatable;

    let stores = TestStores::default();
    let model = Model::create("Sphere".to_string());

    let expected = "struct VertexInput {\n\
        \u{20}   @location(0) position: vec3<f32>,\n\
        \u{20}   @location(1) texture_coordinates: vec2<f32>,\n\
        \u{20}   @location(2) normal: vec3<f32>,\n\
        \u{20}   @location(3) tangent: vec3<f32>,\n\
        \u{20}   @location(4) bitangent: vec3<f32>,\n\
        }";

    assert_eq!(wgsl(&model, &stores.ctx()), expected);

    // GLSL has no struct vertex inputs; they become global `in` declarations.
    let expected = "layout(location = 0) in vec3 position;\n\
        layout(location = 1) in vec2 texture_coordinates;\n\
        layout(location = 2) in vec3 normal;\n\
        layout(location = 3) in vec3 tangent;\n\
        layout(location = 4) in vec3 bitangent;";

    assert_eq!(glsl(&model, &stores.ctx()), expected);
    assert_glsl_parses(expected, naga::ShaderStage::Vertex);
}
