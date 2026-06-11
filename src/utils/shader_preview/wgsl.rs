use super::backend::ShaderBackend;
use super::ir::{
    Access, BindingKind, Sampled, ScalarKind, ShaderBinding, ShaderStruct, ShaderType, TexDim,
};
use crate::utils::texture_format::TextureFormat;

pub struct WgslBackend;

impl WgslBackend {
    fn scalar(scalar: ScalarKind) -> &'static str {
        match scalar {
            ScalarKind::F32 => "f32",
            ScalarKind::I32 => "i32",
            ScalarKind::U32 => "u32",
        }
    }

    fn dimension(dimension: TexDim) -> &'static str {
        match dimension {
            TexDim::D1 => "1d",
            TexDim::D2 => "2d",
            TexDim::D2Array => "2d_array",
            TexDim::Cube => "cube",
            TexDim::CubeArray => "cube_array",
            TexDim::D3 => "3d",
        }
    }

    fn access(access: Access) -> &'static str {
        match access {
            Access::Read => "read",
            Access::Write => "write",
            Access::ReadWrite => "read_write",
            Access::Atomic => "atomic",
        }
    }

    /// WGSL storage texel formats match the lower-cased format variant names
    /// (e.g. `Rgba8Unorm` → `rgba8unorm`); fall back to a common default.
    fn texel_format(format: Option<TextureFormat>) -> String {
        format.map_or_else(
            || "rgba8unorm".to_string(),
            |format| format!("{format:?}").to_lowercase(),
        )
    }
}

impl ShaderBackend for WgslBackend {
    fn format_type(&self, ty: &ShaderType) -> String {
        match ty {
            ShaderType::Scalar(scalar) => Self::scalar(*scalar).to_string(),
            ShaderType::Vector { size, scalar } => format!("vec{size}<{}>", Self::scalar(*scalar)),
            ShaderType::Matrix { cols, rows, scalar } => {
                format!("mat{cols}x{rows}<{}>", Self::scalar(*scalar))
            }
            ShaderType::Texture {
                dim,
                sampled: Sampled::Depth,
            } => format!("texture_depth_{}", Self::dimension(*dim)),
            ShaderType::Texture { dim, sampled } => {
                let element = match sampled {
                    Sampled::Float => "f32",
                    Sampled::Sint => "i32",
                    Sampled::Uint => "u32",
                    Sampled::Depth => unreachable!("handled above"),
                };
                format!("texture_{}<{element}>", Self::dimension(*dim))
            }
            ShaderType::StorageTexture {
                dim,
                format,
                access,
            } => format!(
                "texture_storage_{}<{}, {}>",
                Self::dimension(*dim),
                Self::texel_format(*format),
                Self::access(*access),
            ),
            ShaderType::Sampler { comparison: true } => "sampler_comparison".to_string(),
            ShaderType::Sampler { comparison: false } => "sampler".to_string(),
            ShaderType::Struct(name) => name.clone(),
        }
    }

    fn format_struct(&self, definition: &ShaderStruct) -> String {
        let mut out = format!("struct {} {{\n", definition.name);
        for field in &definition.fields {
            let location = field
                .location
                .map(|location| format!("@location({location}) "))
                .unwrap_or_default();
            out.push_str(&format!(
                "    {location}{}: {},\n",
                field.name,
                self.format_type(&field.ty)
            ));
        }
        out.push('}');
        out
    }

    fn format_binding(&self, binding: &ShaderBinding) -> String {
        let group = binding
            .group
            .map_or_else(|| "_".to_string(), |group| group.to_string());
        let prefix = format!("@group({group}) @binding({})", binding.binding);
        let ty = self.format_type(&binding.ty);
        match binding.kind {
            BindingKind::Uniform => format!("{prefix} var<uniform> {}: {ty};", binding.name),
            _ => format!("{prefix} var {}: {ty};", binding.name),
        }
    }
}
