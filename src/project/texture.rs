use crate::{
    texture,
    ui::{self},
};

pub struct TextureEntry {
    pub name: String,
    pub texture: texture::Texture,
    pub texture_format: wgpu::TextureFormat,
    pub egui_id: egui::TextureId,
    size: ui::Size2d,
}

#[allow(dead_code)]
impl TextureEntry {
    pub fn new(
        name: impl Into<String>,
        device: &wgpu::Device,
        size: ui::Size2d,
        texture_format: wgpu::TextureFormat,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) -> TextureEntry {
        let name = name.into();

        let texture = texture::Texture::create_2d_texture(device, &name, size, texture_format);

        let egui_id =
            egui_renderer.register_egui_texture(device, &texture.view, wgpu::FilterMode::Linear);

        TextureEntry {
            name,
            texture,
            texture_format,
            egui_id,
            size,
        }
    }

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
