use slotmap::new_key_type;

use crate::project::Project;

new_key_type! {
    pub struct ShaderId;
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
