use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// The texture formats the application supports.
///
/// This is a curated subset of [`wgpu::TextureFormat`] — only the formats the
/// app actually creates and exposes in the UI. Owning the enum (instead of
/// passing the full wgpu format list around) lets us match on it exhaustively
/// and keeps unsupported formats out of the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum TextureFormat {
    Rgba8UnormSrgb,
    Rgba8Unorm,
    Rgba16Float,
    Rgba32Float,
    Depth32Float,
}

impl TextureFormat {
    /// Color formats, usable as render targets and sampled textures.
    pub const COLOR: [TextureFormat; 4] = [
        Self::Rgba8UnormSrgb,
        Self::Rgba8Unorm,
        Self::Rgba16Float,
        Self::Rgba32Float,
    ];

    /// Depth/stencil formats.
    pub const DEPTH: [TextureFormat; 1] = [Self::Depth32Float];

    pub fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            Self::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            Self::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
            Self::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
            Self::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
            Self::Depth32Float => wgpu::TextureFormat::Depth32Float,
        }
    }

    /// Human-readable name shown in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rgba8UnormSrgb => "RGBA8 Unorm sRGB",
            Self::Rgba8Unorm => "RGBA8 Unorm Linear",
            Self::Rgba16Float => "RGBA16 Float",
            Self::Rgba32Float => "RGBA32 Float",
            Self::Depth32Float => "Depth32 Float",
        }
    }
}
