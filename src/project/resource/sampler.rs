use crate::{
    error::AppResult,
    project::{
        Creatable, ProjectResource, SamplerId,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
};

#[derive(Debug, Clone, PartialEq)]
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

pub struct Sampler {
    label: String,
    spec: SamplerSpec,
    revision: Revision,
}

pub struct SamplerRuntime {
    inner: wgpu::Sampler,
}

impl Sampler {
    pub fn new(label: impl Into<String>, spec: SamplerSpec) -> Sampler {
        Sampler {
            label: label.into(),
            spec,
            revision: Revision::default(),
        }
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.revision.increase();
    }

    pub fn spec(&self) -> &SamplerSpec {
        &self.spec
    }

    pub fn set_spec(&mut self, spec: SamplerSpec) {
        self.spec = spec;
        self.spec.lod_max_clamp = self.spec.lod_max_clamp.max(self.spec.lod_min_clamp);
        self.revision.increase();
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
    const DEFAULT_LABEL: &'static str = "Sampler";

    fn create(label: String) -> Self {
        Self::new(label, SamplerSpec::default())
    }
}

impl ProjectResource for Sampler {
    type Id = SamplerId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for Sampler {
    type Context<'a> = &'a wgpu::Device;
    type Runtime = SamplerRuntime;

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let inner = Self::create_sampler(ctx, &self.label, &self.spec);

        Ok(SyncOutcome::Changed(SamplerRuntime { inner }))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }
}
