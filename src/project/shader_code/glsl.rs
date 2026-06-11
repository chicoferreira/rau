use std::collections::HashMap;

use super::backend::ShaderBackend;
use super::ir::{
    Access, BindingKind, Sampled, ScalarKind, ShaderBinding, ShaderItem, ShaderModule,
    ShaderStruct, ShaderType, TexDim,
};
use crate::utils::texture_format::TextureFormat;

/// Renders Vulkan-flavored GLSL
pub struct GlslBackend;

impl GlslBackend {
    fn scalar(scalar: ScalarKind) -> &'static str {
        match scalar {
            ScalarKind::F32 => "float",
            ScalarKind::I32 => "int",
            ScalarKind::U32 => "uint",
        }
    }

    /// Prefix for vector, texture and image type names (`vec3`/`ivec3`/`uvec3`).
    fn scalar_prefix(scalar: ScalarKind) -> &'static str {
        match scalar {
            ScalarKind::F32 => "",
            ScalarKind::I32 => "i",
            ScalarKind::U32 => "u",
        }
    }

    fn dimension(dimension: TexDim) -> &'static str {
        match dimension {
            TexDim::D1 => "1D",
            TexDim::D2 => "2D",
            TexDim::D2Array => "2DArray",
            TexDim::Cube => "Cube",
            TexDim::CubeArray => "CubeArray",
            TexDim::D3 => "3D",
        }
    }

    /// Memory qualifier for storage images; GLSL defaults to read-write when
    /// no qualifier is present.
    fn access(access: Access) -> &'static str {
        match access {
            Access::Read => "readonly ",
            Access::Write => "writeonly ",
            Access::ReadWrite | Access::Atomic => "",
        }
    }

    /// GLSL image format layout qualifier; falls back to a common default.
    /// sRGB and depth formats have no image format equivalent, so they map to
    /// the closest storable one.
    fn texel_format(format: Option<TextureFormat>) -> &'static str {
        match format {
            Some(TextureFormat::Rgba16Float) => "rgba16f",
            Some(TextureFormat::Rgba32Float) => "rgba32f",
            Some(TextureFormat::Depth32Float) => "r32f",
            Some(TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm) | None => "rgba8",
        }
    }

    fn set_qualifier(group: Option<u32>) -> String {
        group.map_or_else(|| "_".to_string(), |group| group.to_string())
    }

    /// Uniforms are emitted as uniform blocks with the struct fields inlined,
    /// so the member access path matches the WGSL output (`name.field`).
    fn format_uniform_block(
        &self,
        binding: &ShaderBinding,
        definition: Option<&ShaderStruct>,
    ) -> String {
        let block_name = self.format_type(&binding.ty);
        let mut out = format!(
            "layout(set = {}, binding = {}) uniform {block_name} {{\n",
            Self::set_qualifier(binding.group),
            binding.binding,
        );
        if let Some(definition) = definition {
            for field in &definition.fields {
                out.push_str(&format!(
                    "    {} {};\n",
                    self.format_type(&field.ty),
                    field.name
                ));
            }
        }
        out.push_str(&format!("}} {};", binding.name));
        out
    }
}

impl ShaderBackend for GlslBackend {
    fn format_type(&self, ty: &ShaderType) -> String {
        match ty {
            ShaderType::Scalar(scalar) => Self::scalar(*scalar).to_string(),
            ShaderType::Vector { size, scalar } => {
                format!("{}vec{size}", Self::scalar_prefix(*scalar))
            }
            // GLSL matrices are float-only; the app only produces f32 matrices.
            ShaderType::Matrix { cols, rows, .. } if cols == rows => format!("mat{cols}"),
            ShaderType::Matrix { cols, rows, .. } => format!("mat{cols}x{rows}"),
            // GLSL has no dedicated depth texture type; depth textures are
            // plain textures sampled through a `samplerShadow`.
            ShaderType::Texture { dim, sampled } => {
                let prefix = match sampled {
                    Sampled::Float | Sampled::Depth => "",
                    Sampled::Sint => "i",
                    Sampled::Uint => "u",
                };
                format!("{prefix}texture{}", Self::dimension(*dim))
            }
            // The format and access qualifiers live in the declaration, not
            // the type name; see `format_binding`. All supported formats are
            // float-sampled, so no i/u prefix is needed.
            ShaderType::StorageTexture { dim, .. } => format!("image{}", Self::dimension(*dim)),
            ShaderType::Sampler { comparison: true } => "samplerShadow".to_string(),
            ShaderType::Sampler { comparison: false } => "sampler".to_string(),
            ShaderType::Struct(name) => name.clone(),
        }
    }

    fn format_struct(&self, definition: &ShaderStruct) -> String {
        let is_vertex_input = definition
            .fields
            .iter()
            .any(|field| field.location.is_some());

        if is_vertex_input {
            definition
                .fields
                .iter()
                .map(|field| {
                    let location = field
                        .location
                        .map(|location| format!("layout(location = {location}) "))
                        .unwrap_or_default();
                    format!(
                        "{location}in {} {};",
                        self.format_type(&field.ty),
                        field.name
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            let mut out = format!("struct {} {{\n", definition.name);
            for field in &definition.fields {
                out.push_str(&format!(
                    "    {} {};\n",
                    self.format_type(&field.ty),
                    field.name
                ));
            }
            out.push_str("};");
            out
        }
    }

    fn format_binding(&self, binding: &ShaderBinding) -> String {
        let set = Self::set_qualifier(binding.group);
        let ty = self.format_type(&binding.ty);
        match binding.kind {
            BindingKind::Uniform => self.format_uniform_block(binding, None),
            BindingKind::StorageTexture => {
                let (format, access) = match &binding.ty {
                    ShaderType::StorageTexture { format, access, .. } => {
                        (Self::texel_format(*format), Self::access(*access))
                    }
                    _ => (Self::texel_format(None), ""),
                };
                format!(
                    "layout(set = {set}, binding = {}, {format}) uniform {access}{ty} {};",
                    binding.binding, binding.name,
                )
            }
            BindingKind::Texture | BindingKind::Sampler => format!(
                "layout(set = {set}, binding = {}) uniform {ty} {};",
                binding.binding, binding.name,
            ),
        }
    }

    fn render(&self, module: &ShaderModule) -> String {
        let definitions: HashMap<&str, &ShaderStruct> = module
            .items()
            .iter()
            .filter_map(|item| match item {
                ShaderItem::Struct(definition) => Some((definition.name.as_str(), definition)),
                _ => None,
            })
            .collect();

        let inlined = |name: &str| {
            module.items().iter().any(|item| match item {
                ShaderItem::Binding(binding) if binding.kind == BindingKind::Uniform => {
                    matches!(&binding.ty, ShaderType::Struct(struct_name) if struct_name == name)
                }
                _ => false,
            })
        };

        let mut structs = Vec::new();
        let mut rest = Vec::new();

        for item in module.items() {
            match item {
                ShaderItem::Struct(definition) => {
                    if !inlined(&definition.name) {
                        structs.push(self.format_struct(definition));
                    }
                }
                ShaderItem::Binding(binding) if binding.kind == BindingKind::Uniform => {
                    let definition = match &binding.ty {
                        ShaderType::Struct(name) => definitions.get(name.as_str()).copied(),
                        _ => None,
                    };
                    rest.push(self.format_uniform_block(binding, definition));
                }
                ShaderItem::Binding(binding) => rest.push(self.format_binding(binding)),
                ShaderItem::Comment(text) => rest.push(self.format_comment(text)),
            }
        }

        let mut blocks = Vec::new();
        if !structs.is_empty() {
            blocks.push(structs.join("\n\n"));
        }
        if !rest.is_empty() {
            blocks.push(rest.join("\n"));
        }
        blocks.join("\n\n")
    }
}
