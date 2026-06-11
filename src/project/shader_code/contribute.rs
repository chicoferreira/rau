use heck::{ToSnakeCase, ToUpperCamelCase};

use super::ir::{
    Access, BindingKind, Sampled, ScalarKind, ShaderBinding, ShaderField, ShaderModule,
    ShaderStruct, ShaderType, TexDim,
};
use crate::project::{
    Project, SamplerId, TextureViewId,
    resource::{
        bindgroup::{BindGroup, BindGroupResource},
        compute_pass::ComputePass,
        model::{Model, vertex_buffer::VertexBufferSpec},
        render_pipeline::{BindGroupTarget, RenderDrawStrategy, RenderPipeline},
        sampler::Sampler,
        texture::Texture,
        texture_view::TextureView,
        uniform::{Uniform, UniformFieldDataKind},
    },
    storage::Storage,
};
use crate::utils::texture_format::TextureFormat;

pub struct ShaderGenCtx<'a> {
    pub bind_groups: &'a Storage<BindGroup>,
    pub uniforms: &'a Storage<Uniform>,
    pub texture_views: &'a Storage<TextureView>,
    pub samplers: &'a Storage<Sampler>,
    pub textures: &'a Storage<Texture>,
    pub models: &'a Storage<Model>,
}

impl<'a> ShaderGenCtx<'a> {
    pub fn from_project(project: &'a Project) -> Self {
        Self {
            bind_groups: &project.bind_groups,
            uniforms: &project.uniforms,
            texture_views: &project.texture_views,
            samplers: &project.samplers,
            textures: &project.textures,
            models: &project.models,
        }
    }
}

pub trait ShaderInterface {
    fn contribute(&self, module: &mut ShaderModule, ctx: &ShaderGenCtx);
}

/// A bind group rendered at a specific `@group` index, or `None` when the index
/// isn't known yet (the bind group inspector, which isn't tied to a slot).
pub struct BindGroupAt<'a> {
    pub group: Option<u32>,
    pub bind_group: &'a BindGroup,
}

impl<'a> BindGroupAt<'a> {
    pub fn new(group: impl Into<Option<u32>>, bind_group: &'a BindGroup) -> Self {
        let group = group.into();
        Self { group, bind_group }
    }
}

impl ShaderInterface for Uniform {
    fn contribute(&self, module: &mut ShaderModule, _ctx: &ShaderGenCtx) {
        module.add_struct(uniform_struct(self));
    }
}

impl ShaderInterface for Model {
    fn contribute(&self, module: &mut ShaderModule, _ctx: &ShaderGenCtx) {
        module.add_struct(vertex_input_struct(self.vertex_buffer_spec()));
    }
}

impl ShaderInterface for BindGroupAt<'_> {
    fn contribute(&self, module: &mut ShaderModule, ctx: &ShaderGenCtx) {
        contribute_bind_group(module, self.group, self.bind_group, ctx);
    }
}

impl ShaderInterface for RenderPipeline {
    fn contribute(&self, module: &mut ShaderModule, ctx: &ShaderGenCtx) {
        // A model draw strategy feeds the vertex shader, so surface its layout.
        let model = match self.draw_strategy() {
            RenderDrawStrategy::Model {
                model_id: Some(model_id),
                ..
            } => ctx.models.get(*model_id).ok(),
            _ => None,
        };
        if let Some(model) = model {
            model.contribute(module, ctx);
        }

        for (group, target) in self.bind_groups().iter().enumerate() {
            let group = group as u32;
            match target {
                BindGroupTarget::Empty => module.comment(format!("group/set {group} is empty")),
                BindGroupTarget::ModelMaterial => {
                    module.comment(format!(
                        "group/set {group} is bound to each mesh's material bind group"
                    ));
                    // Material bind group layouts are validated to match, so the
                    // first one stands in for all of them.
                    let material_bind_group = model.and_then(|model| {
                        model
                            .material_bind_group_ids()
                            .iter()
                            .flatten()
                            .find_map(|id| ctx.bind_groups.get(*id).ok())
                    });
                    if let Some(bind_group) = material_bind_group {
                        contribute_bind_group(module, Some(group), bind_group, ctx);
                    }
                }
                BindGroupTarget::Static(id) => match ctx.bind_groups.get(*id) {
                    Ok(bind_group) => contribute_bind_group(module, Some(group), bind_group, ctx),
                    Err(_) => module.comment(format!("group/set {group} is empty")),
                },
            }
        }
    }
}

impl ShaderInterface for ComputePass {
    fn contribute(&self, module: &mut ShaderModule, ctx: &ShaderGenCtx) {
        for (group, id) in self.bind_groups().iter().enumerate() {
            let group = group as u32;
            match id {
                Some(id) => match ctx.bind_groups.get(*id) {
                    Ok(bind_group) => contribute_bind_group(module, Some(group), bind_group, ctx),
                    Err(_) => module.comment(format!("group {group} is empty")),
                },
                None => module.comment(format!("group {group} is empty")),
            }
        }
    }
}

fn uniform_struct(uniform: &Uniform) -> ShaderStruct {
    ShaderStruct {
        name: uniform.label().to_upper_camel_case(),
        fields: uniform
            .fields()
            .iter()
            .map(|field| ShaderField {
                name: field.label().to_snake_case(),
                ty: field_kind_type(field.kind()),
                location: None,
            })
            .collect(),
    }
}

fn vertex_input_struct(spec: &VertexBufferSpec) -> ShaderStruct {
    ShaderStruct {
        name: "VertexInput".to_string(),
        fields: spec
            .fields
            .iter()
            .enumerate()
            .map(|(location, field)| ShaderField {
                name: field.to_string().to_snake_case(),
                ty: vertex_format_type(field.vertex_format()),
                location: Some(location as u32),
            })
            .collect(),
    }
}

fn contribute_bind_group(
    module: &mut ShaderModule,
    group: Option<u32>,
    bind_group: &BindGroup,
    ctx: &ShaderGenCtx,
) {
    for (binding, entry) in bind_group.entries().iter().enumerate() {
        contribute_entry(module, group, binding as u32, &entry.resource, ctx);
    }
}

fn contribute_entry(
    module: &mut ShaderModule,
    group: Option<u32>,
    binding: u32,
    resource: &BindGroupResource,
    ctx: &ShaderGenCtx,
) {
    let (kind, name, ty) = match *resource {
        BindGroupResource::Uniform(uniform_id) => {
            let (name, ty) = match uniform_id.and_then(|id| ctx.uniforms.get(id).ok()) {
                Some(uniform) => {
                    let definition = uniform_struct(uniform);
                    let ty = ShaderType::Struct(definition.name.clone());
                    module.add_struct(definition);
                    (uniform.label().to_snake_case(), ty)
                }
                None => (
                    format!("binding_{binding}"),
                    ShaderType::Struct("Unknown".to_string()),
                ),
            };
            (BindingKind::Uniform, name, ty)
        }
        BindGroupResource::Texture {
            texture_view_id,
            view_dimension,
            sample_type,
        } => (
            BindingKind::Texture,
            texture_view_var_name(texture_view_id, binding, ctx),
            ShaderType::Texture {
                dim: texture_dimension(view_dimension),
                sampled: sampled_kind(sample_type),
            },
        ),
        BindGroupResource::Sampler {
            sampler_id,
            sampler_binding_type,
        } => (
            BindingKind::Sampler,
            sampler_var_name(sampler_id, binding, ctx),
            ShaderType::Sampler {
                comparison: matches!(sampler_binding_type, wgpu::SamplerBindingType::Comparison),
            },
        ),
        BindGroupResource::StorageTexture {
            texture_view_id,
            access,
            view_dimension,
        } => (
            BindingKind::StorageTexture,
            texture_view_var_name(texture_view_id, binding, ctx),
            ShaderType::StorageTexture {
                dim: texture_dimension(view_dimension),
                format: resolve_texture_format(texture_view_id, ctx),
                access: access_kind(access),
            },
        ),
    };

    module.add_binding(ShaderBinding {
        group,
        binding,
        kind,
        name,
        ty,
    });
}

fn field_kind_type(kind: UniformFieldDataKind) -> ShaderType {
    match kind {
        UniformFieldDataKind::Float => ShaderType::Scalar(ScalarKind::F32),
        UniformFieldDataKind::Vec2f => ShaderType::Vector {
            size: 2,
            scalar: ScalarKind::F32,
        },
        UniformFieldDataKind::Vec3f | UniformFieldDataKind::Rgb => ShaderType::Vector {
            size: 3,
            scalar: ScalarKind::F32,
        },
        UniformFieldDataKind::Vec4f | UniformFieldDataKind::Rgba => ShaderType::Vector {
            size: 4,
            scalar: ScalarKind::F32,
        },
        UniformFieldDataKind::Mat4x4f => ShaderType::Matrix {
            cols: 4,
            rows: 4,
            scalar: ScalarKind::F32,
        },
    }
}

fn vertex_format_type(format: wgpu::VertexFormat) -> ShaderType {
    use wgpu::VertexFormat as Vf;

    let vector = |size, scalar| ShaderType::Vector { size, scalar };
    match format {
        Vf::Float32 => ShaderType::Scalar(ScalarKind::F32),
        Vf::Float32x2 => vector(2, ScalarKind::F32),
        Vf::Float32x3 => vector(3, ScalarKind::F32),
        Vf::Float32x4 => vector(4, ScalarKind::F32),
        Vf::Sint32 => ShaderType::Scalar(ScalarKind::I32),
        Vf::Sint32x2 => vector(2, ScalarKind::I32),
        Vf::Sint32x3 => vector(3, ScalarKind::I32),
        Vf::Sint32x4 => vector(4, ScalarKind::I32),
        Vf::Uint32 => ShaderType::Scalar(ScalarKind::U32),
        Vf::Uint32x2 => vector(2, ScalarKind::U32),
        Vf::Uint32x3 => vector(3, ScalarKind::U32),
        Vf::Uint32x4 => vector(4, ScalarKind::U32),
        // Other formats aren't produced by the vertex buffer spec today.
        _ => ShaderType::Scalar(ScalarKind::F32),
    }
}

fn texture_dimension(dimension: wgpu::TextureViewDimension) -> TexDim {
    match dimension {
        wgpu::TextureViewDimension::D1 => TexDim::D1,
        wgpu::TextureViewDimension::D2 => TexDim::D2,
        wgpu::TextureViewDimension::D2Array => TexDim::D2Array,
        wgpu::TextureViewDimension::Cube => TexDim::Cube,
        wgpu::TextureViewDimension::CubeArray => TexDim::CubeArray,
        wgpu::TextureViewDimension::D3 => TexDim::D3,
    }
}

fn sampled_kind(sample_type: wgpu::TextureSampleType) -> Sampled {
    match sample_type {
        wgpu::TextureSampleType::Float { .. } => Sampled::Float,
        wgpu::TextureSampleType::Sint => Sampled::Sint,
        wgpu::TextureSampleType::Uint => Sampled::Uint,
        wgpu::TextureSampleType::Depth => Sampled::Depth,
    }
}

fn access_kind(access: wgpu::StorageTextureAccess) -> Access {
    match access {
        wgpu::StorageTextureAccess::ReadOnly => Access::Read,
        wgpu::StorageTextureAccess::WriteOnly => Access::Write,
        wgpu::StorageTextureAccess::ReadWrite => Access::ReadWrite,
        wgpu::StorageTextureAccess::Atomic => Access::Atomic,
    }
}

fn texture_view_var_name(id: Option<TextureViewId>, binding: u32, ctx: &ShaderGenCtx) -> String {
    var_name_from_label(
        id.and_then(|id| ctx.texture_views.get_label(id).ok()),
        binding,
    )
}

fn sampler_var_name(id: Option<SamplerId>, binding: u32, ctx: &ShaderGenCtx) -> String {
    var_name_from_label(id.and_then(|id| ctx.samplers.get_label(id).ok()), binding)
}

fn var_name_from_label(label: Option<&str>, binding: u32) -> String {
    match label {
        Some(label) if !label.trim().is_empty() => label.to_snake_case(),
        _ => format!("binding_{binding}"),
    }
}

fn resolve_texture_format(id: Option<TextureViewId>, ctx: &ShaderGenCtx) -> Option<TextureFormat> {
    let texture_view = ctx.texture_views.get(id?).ok()?;
    let texture = ctx.textures.get(texture_view.texture_id()?).ok()?;
    Some(texture.format())
}
