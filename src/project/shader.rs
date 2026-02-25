use anyhow::Context;
use slotmap::new_key_type;

use crate::project::Project;

new_key_type! {
    pub struct ShaderId;
}

impl Project {
    pub fn get_shader(&self, id: ShaderId) -> Option<&Shader> {
        self.shaders.get(id)
    }

    pub fn get_shader_mut(&mut self, id: ShaderId) -> Option<&mut Shader> {
        self.shaders.get_mut(id)
    }

    pub fn register_shader(&mut self, label: impl Into<String>, source: String) -> ShaderId {
        self.shaders.insert(Shader {
            label: label.into(),
            source,
        })
    }

    pub fn list_shaders(&self) -> impl Iterator<Item = (ShaderId, &Shader)> {
        self.shaders.iter()
    }
}

pub struct Shader {
    pub label: String,
    pub source: String,
}

impl Shader {
    pub fn create_wgpu_shader_module(
        &self,
        device: &wgpu::Device,
    ) -> anyhow::Result<wgpu::ShaderModule> {
        let module =
            wgpu::naga::front::wgsl::parse_str(&self.source).context("Failed to parse shader")?;

        let _module_info: wgpu::naga::valid::ModuleInfo = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            wgpu::naga::valid::Capabilities::all(),
        )
        .subgroup_stages(wgpu::naga::valid::ShaderStages::all())
        .subgroup_operations(wgpu::naga::valid::SubgroupOperationSet::all())
        .validate(&module)
        .context("Failed to validate shader")?;

        Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&self.label),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
        }))
    }
}
