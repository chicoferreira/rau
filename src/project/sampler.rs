use crate::{
    error::{AppResult, WgpuErrorScope},
    project::{
        ProjectResource, SamplerId,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
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
    dirty: bool,
    has_error: bool,
    inner: wgpu::Sampler,
}

impl Sampler {
    pub fn new(device: &wgpu::Device, label: String, spec: SamplerSpec) -> AppResult<Sampler> {
        let sampler = Self::create_sampler(device, &label, &spec)?;

        Ok(Sampler {
            label,
            spec,
            dirty: false,
            has_error: false,
            inner: sampler,
        })
    }

    pub fn inner(&self) -> &wgpu::Sampler {
        &self.inner
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.dirty = true;
    }

    pub fn spec(&self) -> &SamplerSpec {
        &self.spec
    }

    pub fn set_spec(&mut self, spec: SamplerSpec) {
        self.spec = spec;
        self.spec.lod_max_clamp = self.spec.lod_max_clamp.max(self.spec.lod_min_clamp);
        self.dirty = true;
    }

    fn create_sampler(
        device: &wgpu::Device,
        label: &str,
        spec: &SamplerSpec,
    ) -> AppResult<wgpu::Sampler> {
        let scope = WgpuErrorScope::push(device);
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
        scope.pop()?;
        Ok(sampler)
    }
}

impl ProjectResource for Sampler {
    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Sampler {
    type Context<'a> = &'a wgpu::Device;
    type Id = SamplerId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        device: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        if !self.dirty && !self.has_error {
            return Ok(None);
        }

        self.inner = Self::create_sampler(device, &self.label, &self.spec)
            .inspect_err(|_| self.has_error = true)?;

        self.has_error = false;
        self.dirty = false;

        Ok(Some(ProjectEvent::SamplerRecreated(id)))
    }
}
