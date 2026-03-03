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
    fn egui_texture_view(texture: &texture::Texture, label: &str) -> wgpu::TextureView {
        texture.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label} egui texture view")),
            // make this configurable later stating that to get the correct color, egui expects Rgba8Unorm
            format: Some(texture.texture.format().remove_srgb_suffix()),
            ..Default::default()
        })
    }

    pub fn new(
        name: impl Into<String>,
        device: &wgpu::Device,
        size: ui::Size2d,
        texture_format: wgpu::TextureFormat,
        egui_renderer: &mut ui::renderer::EguiRenderer,
    ) -> TextureEntry {
        let name = name.into();

        let texture = texture::Texture::create_2d_texture(device, &name, size, texture_format);
        let egui_texture_view = Self::egui_texture_view(&texture, &name);

        let egui_id = egui_renderer.register_egui_texture(
            device,
            &egui_texture_view,
            wgpu::FilterMode::Linear,
        );

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
        let egui_texture_view = Self::egui_texture_view(&self.texture, &self.name);

        egui_renderer.update_egui_texture(
            device,
            &egui_texture_view,
            wgpu::FilterMode::Linear,
            self.egui_id,
        );
    }
}
