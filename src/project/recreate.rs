use crate::project::{
    BindGroupId, SamplerId, TextureId, TextureViewId, UniformId, ViewportId, storage::Storage,
};

pub struct RecreateTracker {
    recreated_ids: Vec<ProjectResourceId>,
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

pub enum RecreateResult {
    Recreated,
    Unchanged,
}

pub trait Recreatable {
    type Context<'a>;

    fn recreate<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> RecreateResult;
}

impl RecreateTracker {
    pub fn new() -> Self {
        Self {
            recreated_ids: Vec::new(),
        }
    }

    pub fn recreate_if_needed<'ctx, R: Recreatable>(
        &mut self,
        object_id: ProjectResourceId,
        object: &mut R,
        mut project: &mut R::Context<'ctx>,
    ) {
        let recreate_result = object.recreate(&mut project, self);
        if let RecreateResult::Recreated = recreate_result {
            log::debug!("Recreated {object_id:?}");
            self.recreated_ids.push(object_id);
        }
    }

    pub fn recreate_storage<'ctx, R: Recreatable, Id: slotmap::Key + Into<ProjectResourceId>>(
        &mut self,
        storage: &mut Storage<Id, R>,
        project: &mut R::Context<'ctx>,
    ) {
        for (id, object) in storage.list_mut() {
            self.recreate_if_needed(id.into(), object, project);
        }
    }

    pub fn was_recreated(&self, object_id: impl Into<ProjectResourceId>) -> bool {
        self.recreated_ids.contains(&object_id.into())
    }
}
