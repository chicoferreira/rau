use crate::project::{
    SamplerId,
    recreate::{ProjectEvent, Recreatable, RecreateTracker},
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
    pub label: String,
    spec: SamplerSpec,
    dirty: bool,
    inner: wgpu::Sampler,
}

impl Sampler {
    pub fn new(device: &wgpu::Device, label: String, spec: SamplerSpec) -> Sampler {
        let sampler = Self::create_sampler(device, &label, &spec);

        Sampler {
            label,
            spec,
            dirty: false,
            inner: sampler,
        }
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

    fn create_sampler(device: &wgpu::Device, label: &str, spec: &SamplerSpec) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
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
        })
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
    ) -> Option<ProjectEvent> {
        if !self.dirty {
            return None;
        }

        self.dirty = false;
        self.inner = Self::create_sampler(device, &self.label, &self.spec);
        Some(ProjectEvent::SamplerRecreated(id))
    }
}
