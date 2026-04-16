use crate::{
    error::AppResult,
    project::{
        ProjectResource, ShaderId,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
    },
};

pub struct Shader {
    pub label: String,
    source: String,
    dirty: bool,
    inner: wgpu::ShaderModule,
}

impl Shader {
    pub fn new(
        device: &wgpu::Device,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> AppResult<Self> {
        let label = label.into();
        let source = source.into();
        let inner = Self::create_wgpu_shader_module(device, &label, &source)?;

        Ok(Self {
            label,
            source,
            dirty: false,
            inner,
        })
    }

    pub fn inner(&self) -> &wgpu::ShaderModule {
        &self.inner
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.dirty = true;
    }

    fn create_wgpu_shader_module(
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

        Ok(device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Naga(std::borrow::Cow::Owned(module)),
        }))
    }
}

impl ProjectResource for Shader {
    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Shader {
    type Context<'a> = &'a wgpu::Device;

    type Id = ShaderId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        if !self.dirty {
            return Ok(None);
        }

        let inner = Self::create_wgpu_shader_module(ctx, &self.label, &self.source)?;
        self.inner = inner;

        self.dirty = false;

        Ok(Some(ProjectEvent::ShaderRecreated(id)))
    }
}
