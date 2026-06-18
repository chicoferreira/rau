use serde::{Deserialize, Serialize};
use strum::EnumIter;

use crate::error::{AppError, AppResult};

/// The sampler address modes the application supports.
///
/// This is a curated subset of [`wgpu::AddressMode`] — `ClampToBorder` is left
/// out because it is gated behind the `ADDRESS_MODE_CLAMP_TO_BORDER` feature
/// (and needs a border color), which the app never enables. Owning the enum
/// keeps that unsupported mode out of the project entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "camelCase")]
pub enum AddressMode {
    #[default]
    ClampToEdge,
    Repeat,
    MirrorRepeat,
}

impl AddressMode {
    pub fn to_wgpu(self) -> wgpu::AddressMode {
        match self {
            Self::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            Self::Repeat => wgpu::AddressMode::Repeat,
            Self::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }

    /// Human-readable name shown in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::ClampToEdge => "Clamp To Edge",
            Self::Repeat => "Repeat",
            Self::MirrorRepeat => "Mirror Repeat",
        }
    }
}

/// How vertices are assembled into primitives and rasterized.
///
/// This mirrors the subset of [`wgpu::PrimitiveState`] the app exposes. The
/// feature-gated fields are intentionally omitted:
/// - `unclipped_depth` is gated behind the `DEPTH_CLIP_CONTROL` feature.
/// - `conservative` is gated behind the `CONSERVATIVE_RASTERIZATION` feature.
///
/// The app never enables those features, so they could only ever be left at
/// their defaults. Owning the struct keeps them out of the project and the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrimitiveState {
    pub topology: wgpu::PrimitiveTopology,
    pub strip_index_format: Option<wgpu::IndexFormat>,
    pub front_face: wgpu::FrontFace,
    pub cull_mode: Option<wgpu::Face>,
    pub polygon_mode: wgpu::PolygonMode,
}

impl Default for PrimitiveState {
    fn default() -> Self {
        let wgpu::PrimitiveState {
            topology,
            strip_index_format,
            front_face,
            cull_mode,
            polygon_mode,
            ..
        } = wgpu::PrimitiveState::default();
        Self {
            topology,
            strip_index_format,
            front_face,
            cull_mode,
            polygon_mode,
        }
    }
}

impl PrimitiveState {
    pub fn to_wgpu(self) -> wgpu::PrimitiveState {
        wgpu::PrimitiveState {
            topology: self.topology,
            strip_index_format: self.strip_index_format,
            front_face: self.front_face,
            cull_mode: self.cull_mode,
            unclipped_depth: false,
            polygon_mode: self.polygon_mode,
            conservative: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderSourceKind {
    Wgsl,
    Glsl(naga::ShaderStage),
}

impl ShaderSourceKind {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "wgsl" => Some(Self::Wgsl),
            "vert" => Some(Self::Glsl(naga::ShaderStage::Vertex)),
            "frag" => Some(Self::Glsl(naga::ShaderStage::Fragment)),
            "comp" => Some(Self::Glsl(naga::ShaderStage::Compute)),
            _ => None,
        }
    }
}

pub fn compile_shader(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    kind: ShaderSourceKind,
) -> AppResult<wgpu::ShaderModule> {
    let module = match kind {
        ShaderSourceKind::Wgsl => naga::front::wgsl::parse_str(source)
            .map_err(|err| AppError::ShaderParseError(err.emit_to_string(source)))?,
        ShaderSourceKind::Glsl(stage) => naga::front::glsl::Frontend::default()
            .parse(&naga::front::glsl::Options::from(stage), source)
            .map_err(|errors| AppError::ShaderParseError(errors.emit_to_string(source)))?,
    };

    let _module_info: naga::valid::ModuleInfo = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .subgroup_stages(naga::valid::ShaderStages::all())
    .subgroup_operations(naga::valid::SubgroupOperationSet::all())
    .validate(&module)
    .map_err(|err| AppError::ShaderCompilationError(err.emit_to_string(source)))?;

    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
    }))
}

pub fn create_command_encoder(device: &wgpu::Device, label: &str) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
}
