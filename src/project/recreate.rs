use crate::project::{
    BindGroupId, CameraId, SamplerId, TextureId, TextureViewId, storage::Storage,
};

pub struct RecreateTracker {
    events: Vec<ProjectEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProjectEvent {
    BindGroupRecreated(BindGroupId),
    TextureRecreated(TextureId),
    TextureViewRecreated(TextureViewId),
    SamplerRecreated(SamplerId),
    CameraUpdated(CameraId),
}

pub trait Recreatable {
    type Context<'a>;
    type Id;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> Option<ProjectEvent>;
}

impl RecreateTracker {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn recreate_if_needed<'ctx, R: Recreatable>(
        &mut self,
        id: R::Id,
        object: &mut R,
        mut project: &mut R::Context<'ctx>,
    ) {
        let recreate_result = object.recreate(id, &mut project, self);
        if let Some(event) = recreate_result {
            log::debug!("Recreated: {:?}", event);
            self.events.push(event);
        }
    }

    pub fn recreate_storage<'ctx, R: Recreatable>(
        &mut self,
        storage: &mut Storage<R::Id, R>,
        project: &mut R::Context<'ctx>,
    ) where
        R::Id: slotmap::Key,
    {
        for (id, object) in storage.list_mut() {
            self.recreate_if_needed(id, object, project);
        }
    }

    pub fn happened(&self, object_id: impl Into<ProjectEvent>) -> bool {
        self.events.contains(&object_id.into())
    }
}
