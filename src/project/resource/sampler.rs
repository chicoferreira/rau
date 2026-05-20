use std::task::Poll;

use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    project::{
        Creatable, ProjectResource, SamplerId,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SamplerSpec {
    pub address_mode: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::MipmapFilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
}

impl Default for SamplerSpec {
    fn default() -> Self {
        SamplerSpec {
            address_mode: Default::default(),
            mag_filter: Default::default(),
            min_filter: Default::default(),
            mipmap_filter: Default::default(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sampler {
    label: String,
    spec: SamplerSpec,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct SamplerRuntime {
    inner: wgpu::Sampler,
}

impl Sampler {
    pub fn new(label: impl Into<String>, spec: SamplerSpec) -> Sampler {
        Sampler {
            label: label.into(),
            spec,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn spec(&self) -> &SamplerSpec {
        &self.spec
    }

    pub fn set_spec(&mut self, spec: SamplerSpec) {
        self.spec = spec;
        self.spec.lod_max_clamp = self.spec.lod_max_clamp.max(self.spec.lod_min_clamp);
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    fn create_sampler(device: &wgpu::Device, label: &str, spec: &SamplerSpec) -> wgpu::Sampler {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: spec.address_mode,
            address_mode_v: spec.address_mode,
            address_mode_w: spec.address_mode,
            mag_filter: spec.mag_filter,
            min_filter: spec.min_filter,
            mipmap_filter: spec.mipmap_filter,
            lod_min_clamp: spec.lod_min_clamp,
            lod_max_clamp: spec.lod_max_clamp,
            compare: spec.compare,
            ..Default::default()
        });
        sampler
    }
}

impl SamplerRuntime {
    pub fn inner(&self) -> &wgpu::Sampler {
        &self.inner
    }
}

impl Creatable for Sampler {
    fn create(label: String) -> Self {
        Self::new(label, SamplerSpec::default())
    }
}

impl ProjectResource for Sampler {
    type Id = SamplerId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

#[derive(Default)]
pub enum SamplerJob {
    #[default]
    Start,
    Validation(SamplerRuntime, AsyncJob<AppResult<()>>),
}

impl SyncResource for Sampler {
    type Context<'a> = &'a wgpu::Device;
    type Runtime = SamplerRuntime;
    type Job = SamplerJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        match job {
            SamplerJob::Start => {
                let scope = WgpuErrorScope::push(ctx);
                let inner = Self::create_sampler(ctx, &self.label, &self.spec);

                let runtime = SamplerRuntime { inner };
                self.sync(ctx, None, SamplerJob::Validation(runtime, scope.pop()))
            }
            SamplerJob::Validation(runtime, mut future) => match future.try_resolve() {
                Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                Poll::Pending => Ok(SyncOutcome::Pending(SamplerJob::Validation(
                    runtime, future,
                ))),
            },
        }
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }
}
