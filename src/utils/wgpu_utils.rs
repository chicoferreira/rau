use crate::error::{AppError, AppResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShaderSourceKind {
    Wgsl,
    Glsl(naga::ShaderStage),
}

impl ShaderSourceKind {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "wgsl" => Some(Self::Wgsl),
            "vert" => Some(Self::Glsl(naga::ShaderStage::Vertex)),
            "frag" => Some(Self::Glsl(naga::ShaderStage::Fragment)),
            "comp" => Some(Self::Glsl(naga::ShaderStage::Compute)),
            _ => None,
        }
    }
}

pub fn compile_shader(
    device: &wgpu::Device,
    label: &str,
    source: &str,
    kind: ShaderSourceKind,
) -> AppResult<wgpu::ShaderModule> {
    let module = match kind {
        ShaderSourceKind::Wgsl => naga::front::wgsl::parse_str(source)
            .map_err(|err| AppError::ShaderParseError(err.emit_to_string(source)))?,
        ShaderSourceKind::Glsl(stage) => naga::front::glsl::Frontend::default()
            .parse(&naga::front::glsl::Options::from(stage), source)
            .map_err(|errors| AppError::ShaderParseError(errors.emit_to_string(source)))?,
    };

    let _module_info: naga::valid::ModuleInfo = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .subgroup_stages(naga::valid::ShaderStages::all())
    .subgroup_operations(naga::valid::SubgroupOperationSet::all())
    .validate(&module)
    .map_err(|err| AppError::ShaderCompilationError(err.emit_to_string(source)))?;

    Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
    }))
}

pub fn create_command_encoder(device: &wgpu::Device, label: &str) -> wgpu::CommandEncoder {
    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
}
