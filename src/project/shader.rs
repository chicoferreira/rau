use crate::{error::AppResult, project::ProjectResource};

pub struct Shader {
    pub label: String,
    pub source: String,
}

impl Shader {
    pub fn new(label: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            source: source.into(),
        }
    }

    pub fn create_wgpu_shader_module(
        &self,
        device: &wgpu::Device,
    ) -> AppResult<wgpu::ShaderModule> {
        let module = naga::front::wgsl::parse_str(&self.source)?;

        let _module_info: naga::valid::ModuleInfo = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .subgroup_stages(naga::valid::ShaderStages::all())
        .subgroup_operations(naga::valid::SubgroupOperationSet::all())
        .validate(&module)?;

        Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&self.label),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
        }))
    }
}

impl ProjectResource for Shader {
    fn label(&self) -> &str {
        &self.label
    }
}
