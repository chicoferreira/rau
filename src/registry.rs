use slotmap::{SlotMap, new_key_type};

use crate::{
    texture,
    ui::{self, Size2d},
};

new_key_type! {
    pub struct TextureId;
}

pub struct TextureEntry {
    name: String,
    texture: texture::Texture,
    texture_format: wgpu::TextureFormat,
    egui_id: egui::TextureId,
    size: ui::Size2d,
}

#[allow(dead_code)]
impl TextureEntry {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn egui_id(&self) -> egui::TextureId {
        self.egui_id
    }

    pub fn size(&self) -> Size2d {
        self.size
    }

    pub fn texture(&self) -> &texture::Texture {
        &self.texture
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture_format
    }

    pub fn resize(
        &mut self,
        size: ui::Size2d,
        device: &wgpu::Device,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        self.size = size;
        self.texture = create_texture(device, &self.name, size, self.texture_format);

        egui_renderer.update_egui_texture(
            device,
            &self.texture.view,
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
    }
}

pub struct TextureRegistry {
    textures: SlotMap<TextureId, TextureEntry>,
}

fn create_texture(
    device: &wgpu::Device,
    name: &str,
    size: ui::Size2d,
    texture_format: wgpu::TextureFormat,
) -> texture::Texture {
    texture::Texture::create_texture(
        device,
        Some(name),
        wgpu::Extent3d {
            width: size.width(),
            height: size.height(),
            depth_or_array_layers: 1,
        },
        texture_format.remove_srgb_suffix(),
        &[texture_format.add_srgb_suffix()],
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        wgpu::TextureDimension::D2,
        wgpu::FilterMode::Linear,
    )
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self {
            textures: SlotMap::with_key(),
        }
    }

    pub fn register(
        &mut self,
        name: impl Into<String>,
        device: &wgpu::Device,
        size: ui::Size2d,
        texture_format: wgpu::TextureFormat,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) -> TextureId {
        let name = name.into();

        let texture = create_texture(device, &name, size, texture_format);

        let egui_id =
            egui_renderer.register_egui_texture(device, &texture.view, wgpu::FilterMode::Linear);

        let entry = TextureEntry {
            name,
            texture,
            texture_format,
            egui_id,
            size,
        };

        self.textures.insert(entry)
    }

    pub fn get(&self, id: TextureId) -> Option<&TextureEntry> {
        self.textures.get(id)
    }

    pub fn get_mut(&mut self, id: TextureId) -> Option<&mut TextureEntry> {
        self.textures.get_mut(id)
    }

    pub fn list(&self) -> impl Iterator<Item = (TextureId, &TextureEntry)> {
        self.textures.iter()
    }
}

impl Default for TextureRegistry {
    fn default() -> Self {
        Self::new()
    }
}
