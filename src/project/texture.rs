use crate::{
    texture,
    ui::{self},
};

pub struct TextureEntry {
    pub name: String,
    pub texture: texture::Texture,
    pub egui_id: egui::TextureId,
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
            egui_id,
        }
    }

    pub fn resize(
        &mut self,
        size: ui::Size2d,
        device: &wgpu::Device,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) {
        let texture_format = self.texture.texture.format();
        self.texture =
            texture::Texture::create_2d_texture(device, &self.name, size, texture_format);

        egui_renderer.update_egui_texture(
            device,
            &self.texture.view,
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
    }
}
