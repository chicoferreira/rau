use std::task::Poll;

use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    file::file_storage::FileStorage,
    project::{
        Creatable, ProjectResource, ShaderId,
        paths::FilePath,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    utils::{self, async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shader {
    label: String,
    #[serde(default)]
    source: Option<FilePath>,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct ShaderRuntime {
    inner: wgpu::ShaderModule,
}

#[derive(Default)]
pub enum ShaderJob {
    #[default]
    Start,
    ReadingSource(AsyncJob<AppResult<String>>),
    Validation(ShaderRuntime, AsyncJob<AppResult<()>>),
}

impl Shader {
    pub fn new(label: impl Into<String>, source: FilePath) -> Self {
        let label = label.into();

        Self {
            label,
            source: Some(source),
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    resource_getters! {
        pub fn source() -> Option<&FilePath>;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_label(label: String);
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_source(source: Option<FilePath>);
    }
}

impl ShaderRuntime {
    pub fn inner(&self) -> &wgpu::ShaderModule {
        &self.inner
    }
}

impl Creatable for Shader {
    fn create(label: String) -> Self {
        Self {
            label,
            source: None,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }
}

impl ProjectResource for Shader {
    type Id = ShaderId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

pub struct ShaderCreationContext<'a> {
    pub device: &'a wgpu::Device,
    pub file_storage: &'a FileStorage,
}

impl SyncResource for Shader {
    type Context<'a> = ShaderCreationContext<'a>;
    type Runtime = ShaderRuntime;
    type Job = ShaderJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        let source = match job {
            ShaderJob::Start => {
                let source = self
                    .source
                    .as_ref()
                    .ok_or(AppError::uninit_field("Source"))?;
                let read_job = ctx.file_storage.read_to_string(source);
                return self.sync(ctx, None, ShaderJob::ReadingSource(read_job));
            }
            ShaderJob::ReadingSource(mut future) => match future.try_resolve() {
                Poll::Ready(result) => result?,
                Poll::Pending => return Ok(SyncOutcome::Pending(ShaderJob::ReadingSource(future))),
            },
            ShaderJob::Validation(runtime, mut future) => {
                return match future.try_resolve() {
                    Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                    Poll::Pending => {
                        Ok(SyncOutcome::Pending(ShaderJob::Validation(runtime, future)))
                    }
                };
            }
        };

        let scope = WgpuErrorScope::push(ctx.device);
        let inner = utils::shader::compile_wgsl_shader(ctx.device, &self.label, &source)?;

        let runtime = ShaderRuntime { inner };
        self.sync(ctx, None, ShaderJob::Validation(runtime, scope.pop()))
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        self.source
            .as_ref()
            .is_some_and(|source| tracker.file_changed(source))
    }
}
