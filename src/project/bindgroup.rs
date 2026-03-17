use egui_dnd::utils::shift_vec;

use crate::project::{
    BindGroupId, Project, SamplerId, TextureViewId, UniformId,
    recreate::{ProjectEvent, Recreatable, RecreateTracker},
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

pub type BindGroupEntryId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupEntry {
    // Used for stability in bind group entry reordering
    pub id: BindGroupEntryId,
    pub resource: BindGroupResource,
}

impl BindGroupEntry {
    pub fn new(resource: BindGroupResource) -> Self {
        Self {
            id: fastrand::usize(..),
            resource,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindGroupResource {
    Texture {
        texture_view_id: Option<TextureViewId>,
        // These two fields are used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        view_dimension: wgpu::TextureViewDimension,
        sample_type: wgpu::TextureSampleType,
    },
    Sampler {
        sampler_id: Option<SamplerId>,
        // This field is used on the layout creation
        // TODO: Decide if we keep this here, or move it to the TextureViewId, or separate the layout from the BindGroup
        sampler_binding_type: wgpu::SamplerBindingType,
    },
    Uniform(Option<UniformId>),
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
        let (layout_entries, group_entries): (Vec<_>, Vec<_>) = entries
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, entry)| {
                let group_entry = entry.into_bind_group_entry(index as u32, ctx)?;
                let layout_entry = entry.into_bind_group_layout_entry(index as u32);
                Some((layout_entry, group_entry))
            })
            .unzip();

        let device = ctx.device;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(&format!("{label} Layout")),
            entries: &layout_entries,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &layout,
            entries: &group_entries,
        });

        (layout, bind_group)
    }

    pub fn reorder_entries(&mut self, from: usize, to: usize) {
        if from == to {
            return;
        }
        shift_vec(from, to, &mut self.entries);
        self.dirty = true;
    }
}

impl BindGroupEntry {
    pub fn into_bind_group_entry<'a>(
        &self,
        binding: u32,
        project: &'a BindGroupCreationContext<'a>,
    ) -> Option<wgpu::BindGroupEntry<'a>> {
        let resource = match self.resource {
            BindGroupResource::Texture {
                texture_view_id, ..
            } => {
                let texture_view = project
                    .texture_views
                    .get(texture_view_id?)
                    .expect("deal with this later");
                wgpu::BindingResource::TextureView(texture_view.inner())
            }
            BindGroupResource::Sampler { sampler_id, .. } => {
                let sampler = project
                    .samplers
                    .get(sampler_id?)
                    .expect("deal with this later");
                wgpu::BindingResource::Sampler(sampler.inner())
            }
            BindGroupResource::Uniform(uniform_id) => {
                let uniform = project
                    .uniforms
                    .get(uniform_id?)
                    .expect("deal with this later");

                uniform.buffer().as_entire_binding()
            }
        };

        Some(wgpu::BindGroupEntry { binding, resource })
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
                texture_view_id: Some(texture_view_id),
                ..
            } => tracker.happened(ProjectEvent::TextureViewRecreated(texture_view_id)),
            BindGroupResource::Sampler {
                sampler_id: Some(sampler_id),
                ..
            } => tracker.happened(ProjectEvent::SamplerRecreated(sampler_id)),
            BindGroupResource::Uniform(Some(uniform_id)) => {
                tracker.happened(ProjectEvent::UniformRecreated(uniform_id))
            }
            _ => false,
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
    type Id = BindGroupId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> Option<ProjectEvent> {
        let resources_recreated = self
            .entries
            .iter()
            .any(|entry| entry.resource_recreated(tracker));
        if !self.dirty && !resources_recreated {
            return None;
        }

        let (layout, inner) = Self::create_layout_and_bind_group(ctx, &self.label, &self.entries);

        self.layout = layout;
        self.inner = inner;
        Some(ProjectEvent::BindGroupRecreated(id))
    }
}
