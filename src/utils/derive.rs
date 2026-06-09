//! Shared helpers for deriving textures and texture views from existing
//! resources or image paths.

use crate::{
    project::{
        Project, ProjectResource, TextureId, TextureViewId,
        paths::FilePath,
        resource::{
            model::TextureType,
            texture::{Texture, TextureSource},
            texture_view::TextureView,
        },
    },
    utils::texture_format::TextureFormat,
};

/// Registers a new texture sourced from the image at `path`, deduplicating
/// `label_base` against the existing textures.
pub fn derive_texture(
    project: &mut Project,
    label_base: &str,
    format: TextureFormat,
    path: FilePath,
) -> TextureId {
    let label = project.textures.next_label(label_base);
    let texture = Texture::new(
        label,
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        TextureSource::Image(Some(path)),
    );
    project.textures.register(texture)
}

/// Registers a new texture view backed by `texture_id`, labeled after the
/// texture it views.
pub fn derive_texture_view(project: &mut Project, texture_id: TextureId) -> TextureViewId {
    let texture_label = project
        .textures
        .get(texture_id)
        .map(|texture| texture.label().to_string())
        .unwrap_or_default();
    let label = project
        .texture_views
        .next_label(&format!("{texture_label} View"));
    project
        .texture_views
        .register(TextureView::new(label, Some(texture_id), None, None))
}

/// Derives a texture from a material image `path`, choosing a default format
/// and label for the given `texture_type`.
pub fn derive_texture_from_material_path(
    project: &mut Project,
    path: FilePath,
    texture_type: TextureType,
) -> TextureId {
    let format = default_texture_format(texture_type);
    let label = texture_label_from_path(&path, || texture_type.to_string());
    derive_texture(project, &label, format, path)
}

/// Derives a texture label from an image path, falling back to `fallback` when
/// the path has no usable file stem.
pub fn texture_label_from_path(path: &FilePath, fallback: impl FnOnce() -> String) -> String {
    path.file_stem()
        .filter(|stem| !stem.is_empty())
        .map(str::to_string)
        .unwrap_or_else(fallback)
}

/// Color textures get an srgb format, while data textures (normals, etc.)
/// get a linear one.
pub fn default_texture_format(texture_type: TextureType) -> TextureFormat {
    match texture_type {
        TextureType::Ambient | TextureType::Diffuse | TextureType::Specular => {
            TextureFormat::Rgba8UnormSrgb
        }
        TextureType::Normal | TextureType::Shininess | TextureType::Dissolve => {
            TextureFormat::Rgba8Unorm
        }
    }
}
