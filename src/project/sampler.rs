pub struct Sampler {
    pub label: String,
    pub address_mode: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::MipmapFilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
    dirty: bool,
    inner: wgpu::Sampler,
}

impl Sampler {
    pub fn new(
        device: &wgpu::Device,
        label: String,
        address_mode: wgpu::AddressMode,
        mag_filter: wgpu::FilterMode,
        min_filter: wgpu::FilterMode,
        mipmap_filter: wgpu::MipmapFilterMode,
        lod_min_clamp: f32,
        lod_max_clamp: f32,
        compare: Option<wgpu::CompareFunction>,
    ) -> Sampler {
        let sampler = Self::create_sampler(
            device,
            &label,
            address_mode,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
        );

        Sampler {
            label,
            address_mode,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
            dirty: false,
            inner: sampler,
        }
    }

    pub fn inner(&self) -> &wgpu::Sampler {
        &self.inner
    }

    pub fn update_on_next_frame(&mut self) {
        self.dirty = true;
    }

    // TODO: only needs updating when any of these parameters change
    pub fn update(&mut self, device: &wgpu::Device) {
        if self.dirty {
            self.inner = Self::create_sampler(
                device,
                &self.label,
                self.address_mode,
                self.mag_filter,
                self.min_filter,
                self.mipmap_filter,
                self.lod_min_clamp,
                self.lod_max_clamp,
                self.compare,
            );
        }
    }

    fn create_sampler(
        device: &wgpu::Device,
        label: &str,
        address_mode: wgpu::AddressMode,
        mag_filter: wgpu::FilterMode,
        min_filter: wgpu::FilterMode,
        mipmap_filter: wgpu::MipmapFilterMode,
        lod_min_clamp: f32,
        lod_max_clamp: f32,
        compare: Option<wgpu::CompareFunction>,
    ) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
            ..Default::default()
        })
    }
}
