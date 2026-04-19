use crate::{
    error::AppResult,
    project::{
        ProjectResource, ShaderId,
        recreate::{Recreatable, RecreateTracker, Revision, SyncOutcome},
    },
    utils,
};

pub struct Shader {
    pub label: String,
    source: String,
    revision: Revision,
}

pub struct ShaderRuntime {
    inner: wgpu::ShaderModule,
}

impl Shader {
    pub fn new(label: impl Into<String>, source: impl Into<String>) -> Self {
        let label = label.into();
        let source = source.into();

        Self {
            label,
            source,
            revision: Revision::default(),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn set_source(&mut self, source: impl Into<String>) {
        self.source = source.into();
        self.revision.increase();
    }
}

impl ShaderRuntime {
    pub fn inner(&self) -> &wgpu::ShaderModule {
        &self.inner
    }
}

impl ProjectResource for Shader {
    type Id = ShaderId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Shader {
    type Context<'a> = &'a wgpu::Device;
    type Runtime = ShaderRuntime;

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let inner = utils::shader::compile_wgsl_shader(ctx, &self.label, &self.source)?;

        Ok(SyncOutcome::Recreated(ShaderRuntime { inner }))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, _: &RecreateTracker) -> bool {
        false
    }
}
