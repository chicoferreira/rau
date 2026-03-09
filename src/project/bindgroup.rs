use crate::project::{
    Project, SamplerId, TextureViewId, UniformId,
    recreate::{Recreatable, RecreateResult, RecreateTracker},
    sampler::Sampler,
    storage::Storage,
    texture_view::TextureView,
    uniform::Uniform,
};

pub struct BindGroupCreationContext<'a> {
    pub uniforms: &'a Storage<UniformId, Uniform>,
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
    pub samplers: &'a Storage<SamplerId, Sampler>,
    pub device: &'a wgpu::Device,
}

pub struct BindGroup {
    pub label: String,
    layout: wgpu::BindGroupLayout,
    inner: wgpu::BindGroup,
    entries: Vec<BindGroupEntry>,
    dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupEntry {
    pub resource: BindGroupResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindGroupResource {
    Texture {
        texture_view_id: TextureViewId,
        // These two fields are used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        view_dimension: wgpu::TextureViewDimension,
        // TODO: As when [`wgpu::wgt::TextureSampleType::Float::filterable`] is true it accepts both, maybe we can hardcode it to always be true
        sample_type: wgpu::TextureSampleType,
    },
    Sampler {
        sampler_id: SamplerId,
        // This field is used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        sampler_binding_type: wgpu::SamplerBindingType,
    },
    Uniform(UniformId),
}

impl BindGroup {
    pub fn new(
        project: &Project,
        device: &wgpu::Device,
        label: String,
        entries: Vec<BindGroupEntry>,
    ) -> BindGroup {
        let ctx = &BindGroupCreationContext {
            uniforms: &project.uniforms,
            texture_views: &project.texture_views,
            samplers: &project.samplers,
            device,
        };

        let (layout, inner) = Self::create_layout_and_bind_group(ctx, &label, &entries);

        BindGroup {
            label,
            layout,
            inner,
            entries,
            dirty: false,
        }
    }

    pub fn inner_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn inner(&self) -> &wgpu::BindGroup {
        &self.inner
    }

    pub fn entries(&self) -> &[BindGroupEntry] {
        &self.entries
    }

    pub fn add_entry(&mut self, entry: BindGroupEntry) {
        self.entries.push(entry);
        self.dirty = true;
    }

    pub fn remove_entry(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
            self.dirty = true;
        }
    }

    pub fn update_entry(&mut self, index: usize, entry: BindGroupEntry) {
        if index < self.entries.len() {
            self.entries[index] = entry;
            self.dirty = true;
        }
    }

    fn create_layout_and_bind_group(
        ctx: &BindGroupCreationContext,
        label: &str,
        entries: &[BindGroupEntry],
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout_entries = entries
            .iter()
            .copied()
            .enumerate()
            .map(|(index, entry)| entry.into_bind_group_layout_entry(index as u32))
            .collect::<Vec<_>>();

        let device = ctx.device;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });

        let group_entries = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| entry.into_bind_group_entry(index as u32, ctx))
            .collect::<Vec<_>>();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &group_entries,
        });

        (layout, bind_group)
    }
}

impl BindGroupEntry {
    pub fn into_bind_group_entry<'a>(
        &self,
        binding: u32,
        project: &'a BindGroupCreationContext<'a>,
    ) -> wgpu::BindGroupEntry<'a> {
        let resource = match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            } => {
                let texture_view = project
                    .texture_views
                    .get(texture_view_id)
                    .expect("deal with this later");
                wgpu::BindingResource::TextureView(texture_view.inner())
            }
            BindGroupResource::Sampler { sampler_id, .. } => {
                let sampler = project
                    .samplers
                    .get(sampler_id)
                    .expect("deal with this later");
                wgpu::BindingResource::Sampler(sampler.inner())
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform = project
                    .uniforms
                    .get(uniform_id)
                    .expect("deal with this later");

                uniform.buffer().as_entire_binding()
            }
        };

        wgpu::BindGroupEntry { binding, resource }
    }

    fn into_bind_group_layout_entry(&self, binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: self.resource.into(),
            count: None,
        }
    }

    fn resource_recreated(&self, tracker: &RecreateTracker) -> bool {
        match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            } => tracker.was_recreated(texture_view_id),
            BindGroupResource::Sampler { sampler_id, .. } => tracker.was_recreated(sampler_id),
            BindGroupResource::Uniform(uniform_id) => tracker.was_recreated(uniform_id),
        }
    }
}

impl From<BindGroupResource> for wgpu::BindingType {
    fn from(value: BindGroupResource) -> Self {
        match value {
            BindGroupResource::Texture {
                view_dimension,
                sample_type,
                ..
            } => wgpu::BindingType::Texture {
                sample_type,
                view_dimension,
                multisampled: false,
            },
            BindGroupResource::Sampler {
                sampler_binding_type,
                ..
            } => wgpu::BindingType::Sampler(sampler_binding_type),
            BindGroupResource::Uniform(_) => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        }
    }
}

impl Recreatable for BindGroup {
    type Context<'a> = BindGroupCreationContext<'a>;

    fn recreate<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> RecreateResult {
        let resources_recreated = self
            .entries
            .iter()
            .any(|entry| entry.resource_recreated(tracker));
        if !self.dirty && !resources_recreated {
            return RecreateResult::Unchanged;
        }

        let (layout, inner) = Self::create_layout_and_bind_group(ctx, &self.label, &self.entries);

        self.layout = layout;
        self.inner = inner;
        RecreateResult::Recreated
    }
}
