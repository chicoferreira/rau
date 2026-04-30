use crate::{
    error::AppResult,
    project::{
        ProjectResource, ShaderId,
        file::{FileSystem, ProjectFilePath},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils,
};

pub struct Shader {
    pub label: String,
    source: ProjectFilePath,
    revision: Revision,
}

pub struct ShaderRuntime {
    inner: wgpu::ShaderModule,
}

impl Shader {
    pub fn new(label: impl Into<String>, source: ProjectFilePath) -> Self {
        let label = label.into();

        Self {
            label,
            source,
            revision: Revision::default(),
        }
    }

    pub fn source(&self) -> &ProjectFilePath {
        &self.source
    }

    pub fn set_source(&mut self, source: ProjectFilePath) {
        self.source = source;
        self.revision.increase();
    }
}

impl ShaderRuntime {
    pub fn inner(&self) -> &wgpu::ShaderModule {
        &self.inner
    }
}

// impl Creatable for Shader {
//     const DEFAULT_LABEL: &'static str = "Shader";

//     fn create(label: String) -> Self {
//         const DEFAULT_SOURCE: &str = r#"@vertex
// fn vs_main() -> @builtin(position) vec4<f32> {
//     return vec4<f32>(0.0, 0.0, 0.0, 1.0);
// }

// @fragment
// fn fs_main() -> @location(0) vec4<f32> {
//     return vec4<f32>(1.0, 1.0, 1.0, 1.0);
// }
// "#;

//         Self::new(label, DEFAULT_SOURCE)
//     }
// }

impl ProjectResource for Shader {
    type Id = ShaderId;

    fn label(&self) -> &str {
        &self.label
    }
}

pub struct ShaderCreationContext<'a> {
    pub device: &'a wgpu::Device,
    pub file_system: &'a FileSystem,
}

impl SyncResource for Shader {
    type Context<'a> = ShaderCreationContext<'a>;
    type Runtime = ShaderRuntime;

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let source = ctx.file_system.read_to_string(&self.source)?;
        let inner = utils::shader::compile_wgsl_shader(ctx.device, &self.label, &source)?;

        Ok(SyncOutcome::Changed(ShaderRuntime { inner }))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        tracker.file_changed(&self.source)
    }
}
