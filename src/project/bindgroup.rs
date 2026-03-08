use crate::project::{
    Project, SamplerId, TextureViewId, UniformId,
    recreate::{Recreatable, RecreateResult, RecreateTracker},
    sampler::Sampler,
    storage::Storage,
    texture_view::TextureView,
    uniform::Uniform,
};

pub struct BindGroupProjectView<'a> {
    pub uniforms: &'a Storage<UniformId, Uniform>,
    pub texture_views: &'a Storage<TextureViewId, TextureView>,
    pub samplers: &'a Storage<SamplerId, Sampler>,
}

pub struct BindGroup {
    pub label: String,
    layout: wgpu::BindGroupLayout,
    inner: wgpu::BindGroup,
    pub entries: Vec<BindGroupEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupEntry {
    pub binding: u32,
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
        let view = &BindGroupProjectView {
            uniforms: &project.uniforms,
            texture_views: &project.texture_views,
            samplers: &project.samplers,
        };

        let (layout, inner) = Self::create_layout_and_bind_group(view, &label, &entries, device);

        BindGroup {
            label,
            layout,
            inner,
            entries,
        }
    }

    pub fn inner_layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    pub fn inner(&self) -> &wgpu::BindGroup {
        &self.inner
    }

    fn create_layout_and_bind_group(
        project: &BindGroupProjectView,
        label: &str,
        entries: &[BindGroupEntry],
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let layout_entries = entries.iter().copied().map(Into::into).collect::<Vec<_>>();

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });

        let group_entries = entries
            .iter()
            .map(|entry| entry.into_bind_group_entry(project))
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
        project: &'a BindGroupProjectView<'a>,
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

        wgpu::BindGroupEntry {
            binding: self.binding,
            resource,
        }
    }

    fn is_dirty(&self, tracker: &RecreateTracker) -> bool {
        match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            } => tracker.was_recreated(texture_view_id),
            BindGroupResource::Sampler { sampler_id, .. } => tracker.was_recreated(sampler_id),
            BindGroupResource::Uniform(uniform_id) => tracker.was_recreated(uniform_id),
        }
    }
}

impl From<BindGroupEntry> for wgpu::BindGroupLayoutEntry {
    fn from(value: BindGroupEntry) -> Self {
        wgpu::BindGroupLayoutEntry {
            binding: value.binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: value.resource.into(),
            count: None,
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
    type Context<'a> = BindGroupProjectView<'a>;

    fn recreate<'a>(
        &mut self,
        context: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> RecreateResult {
        let entries_dirty = self.entries.iter().any(|entry| entry.is_dirty(tracker));
        if !entries_dirty {
            return RecreateResult::Unchanged;
        }

        let (layout, inner) =
            Self::create_layout_and_bind_group(context, &self.label, &self.entries, device);

        self.layout = layout;
        self.inner = inner;
        RecreateResult::Recreated
    }
}
