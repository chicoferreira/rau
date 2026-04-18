use crate::error::AppResult;

pub fn compile_wgsl_shader(
    device: &wgpu::Device,
    label: &str,
    source: &str,
) -> AppResult<wgpu::ShaderModule> {
    let module = naga::front::wgsl::parse_str(source)?;

    let _module_info: naga::valid::ModuleInfo = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .subgroup_stages(naga::valid::ShaderStages::all())
    .subgroup_operations(naga::valid::SubgroupOperationSet::all())
    .validate(&module)?;

    // TODO: fetch line errors

    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
    }))
}
