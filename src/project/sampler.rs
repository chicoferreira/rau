use crate::project::recreate::{Recreatable, RecreateResult, RecreateTracker};

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
    pub spec: SamplerSpec,
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
    type Context<'a> = ();

    fn recreate<'a>(
        &mut self,
        _context: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> RecreateResult {
        if !self.dirty {
            return RecreateResult::Unchanged;
        }

        self.inner = Self::create_sampler(device, &self.label, &self.spec);
        RecreateResult::Recreated
    }
}
