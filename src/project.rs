use slotmap::{SlotMap, new_key_type};
use wgpu::util::DeviceExt;

use crate::{
    texture,
    ui::{self, Size2d},
    uniform,
};

new_key_type! {
    pub struct ShaderId;
    pub struct TextureId;
    pub struct UniformId;
    pub struct BindGroupId;
}

#[derive(Default)]
pub struct Project {
    shaders: SlotMap<ShaderId, Shader>,
    textures: SlotMap<TextureId, TextureEntry>,
    uniforms: SlotMap<UniformId, uniform::Uniform>,
    bind_groups: SlotMap<BindGroupId, uniform::BindGroup>,
}

impl Project {
    pub fn get_shader(&self, id: ShaderId) -> Option<&Shader> {
        self.shaders.get(id)
    }

    pub fn register_shader(&mut self, label: impl Into<String>, source: String) -> ShaderId {
        self.shaders.insert(Shader {
            label: label.into(),
            source,
        })
    }

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

    pub fn get_uniform(&self, id: UniformId) -> Option<&uniform::Uniform> {
        self.uniforms.get(id)
    }

    pub fn get_uniform_mut(&mut self, id: UniformId) -> Option<&mut uniform::Uniform> {
        self.uniforms.get_mut(id)
    }

    pub fn list_uniforms(&self) -> impl Iterator<Item = (UniformId, &uniform::Uniform)> {
        self.uniforms.iter()
    }

    pub fn register_uniform(
        &mut self,
        device: &wgpu::Device,
        label: impl Into<String>,
        data: uniform::UniformData,
    ) -> UniformId {
        let label = label.into();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&label),
            contents: &data.cast(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform = uniform::Uniform {
            label,
            data,
            buffer,
        };

        self.uniforms.insert(uniform)
    }

    pub fn register_bind_group(
        &mut self,
        device: &wgpu::Device,
        label: impl Into<String>,
        entries: Vec<uniform::BindGroupEntry>,
    ) -> BindGroupId {
        let label = label.into();

        let layout_entries = entries.iter().copied().map(Into::into).collect::<Vec<_>>();

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });

        let group_entries = entries
            .into_iter()
            .map(|entry| entry.into_bind_group_entry(self))
            .collect::<Vec<_>>();

        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&label),
            layout: &layout,
            entries: &group_entries,
        });

        let bind_group = uniform::BindGroup {
            label,
            layout,
            group,
            entries: Vec::new(),
        };

        self.bind_groups.insert(bind_group)
    }

    pub fn get_bind_group(&self, id: BindGroupId) -> Option<&uniform::BindGroup> {
        self.bind_groups.get(id)
    }
}

pub struct Shader {
    label: String,
    source: String,
}

impl Shader {
    pub fn create_wgpu_shader_module(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&self.label),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&self.source)),
        })
    }
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
