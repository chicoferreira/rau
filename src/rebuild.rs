use crate::project::{
    BindGroupId, SamplerId, TextureId, TextureViewId, UniformId, ViewportId, storage::Storage,
};

pub struct RebuildTracker<'a> {
    recreated_ids: Vec<ProjectResourceId>,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectResourceId {
    BindGroup(BindGroupId),
    Uniform(UniformId),
    Texture(TextureId),
    TextureView(TextureViewId),
    Viewport(ViewportId),
    Sampler(SamplerId),
}

macro_rules! impl_project_resource_from_id {
    ($($id_ty:ty => $variant:ident),+ $(,)?) => {
        $(
            impl From<$id_ty> for ProjectResourceId {
                fn from(value: $id_ty) -> Self {
                    Self::$variant(value)
                }
            }
        )+
    };
}

impl_project_resource_from_id!(
    BindGroupId => BindGroup,
    UniformId => Uniform,
    TextureId => Texture,
    TextureViewId => TextureView,
    ViewportId => Viewport,
    SamplerId => Sampler,
);

pub trait Recreatable {
    type Context<'a>;

    fn should_recreate(&self, context: &Self::Context<'_>, recreate_list: &RebuildTracker) -> bool;

    fn recreate<'a>(
        &mut self,
        context: &mut Self::Context<'a>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
}

impl<'a> RebuildTracker<'a> {
    pub fn new(device: &'a wgpu::Device, queue: &'a wgpu::Queue) -> Self {
        Self {
            recreated_ids: Vec::new(),
            device,
            queue,
        }
    }

    pub fn recreate_if_needed<'ctx, R: Recreatable>(
        &mut self,
        object_id: ProjectResourceId,
        object: &mut R,
        mut project: &mut R::Context<'ctx>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        if object.should_recreate(&project, self) {
            log::debug!("Recreating {object_id:?}");
            object.recreate(&mut project, device, queue);
            self.recreated_ids.push(object_id);
        }
    }

    pub fn recreate_storage<'ctx, R: Recreatable, Id: slotmap::Key + Into<ProjectResourceId>>(
        &mut self,
        storage: &mut Storage<Id, R>,
        project: &mut R::Context<'ctx>,
    ) {
        for (id, object) in storage.list_mut() {
            self.recreate_if_needed(id.into(), object, project, self.device, self.queue);
        }
    }

    pub fn was_recreated(&self, object_id: impl Into<ProjectResourceId>) -> bool {
        self.recreated_ids.contains(&object_id.into())
    }
}
