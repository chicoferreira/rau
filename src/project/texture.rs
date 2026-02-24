use slotmap::new_key_type;

use crate::{
    project::Project,
    texture,
    ui::{self},
};

new_key_type! {
    pub struct TextureId;
}

pub struct TextureEntry {
    pub name: String,
    pub texture: texture::Texture,
    pub texture_format: wgpu::TextureFormat,
    pub egui_id: egui::TextureId,
    size: ui::Size2d,
}

#[allow(dead_code)]
impl TextureEntry {
    pub fn size(&self) -> ui::Size2d {
        self.size
    }

    pub fn resize(
        &mut self,
        size: ui::Size2d,
        device: &wgpu::Device,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        self.size = size;
        self.texture =
            texture::Texture::create_2d_texture(device, &self.name, size, self.texture_format);

        egui_renderer.update_egui_texture(
            device,
            &self.texture.view,
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
    }
}

impl Project {
    pub fn register_texture(
        &mut self,
        name: impl Into<String>,
        device: &wgpu::Device,
        size: ui::Size2d,
        texture_format: wgpu::TextureFormat,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) -> TextureId {
        let name = name.into();

        let texture = texture::Texture::create_2d_texture(device, &name, size, texture_format);

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

    pub fn get_texture(&self, id: TextureId) -> Option<&TextureEntry> {
        self.textures.get(id)
    }

    pub fn get_texture_mut(&mut self, id: TextureId) -> Option<&mut TextureEntry> {
        self.textures.get_mut(id)
    }

    pub fn list_textures(&self) -> impl Iterator<Item = (TextureId, &TextureEntry)> {
        self.textures.iter()
    }
}
