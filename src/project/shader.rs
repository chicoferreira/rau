use anyhow::Context;

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
    ) -> anyhow::Result<wgpu::ShaderModule> {
        let module =
            naga::front::wgsl::parse_str(&self.source).context("Failed to parse shader")?;

        let _module_info: naga::valid::ModuleInfo = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .subgroup_stages(naga::valid::ShaderStages::all())
        .subgroup_operations(naga::valid::SubgroupOperationSet::all())
        .validate(&module)
        .context("Failed to validate shader")?;

        Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&self.label),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
        }))
    }
}

