use crate::{
    error::{AppResult, SourcedError},
    project::{
        BindGroupId, CameraId, ProjectResourceId, SamplerId, TextureId, TextureViewId, UniformId,
        storage::Storage,
    },
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
    UniformRecreated(UniformId),
}

pub trait Recreatable {
    type Context<'a>;
    type Id;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>>;
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
    ) -> AppResult<()> {
        let recreate_result = object.recreate(id, &mut project, self)?;
        if let Some(event) = recreate_result {
            log::debug!("Recreated: {:?}", event);
            self.events.push(event);
        }
        Ok(())
    }

    pub fn recreate_storage<'ctx, R: Recreatable>(
        &mut self,
        storage: &mut Storage<R::Id, R>,
        project: &mut R::Context<'ctx>,
    ) -> Vec<SourcedError>
    where
        R::Id: slotmap::Key + Into<ProjectResourceId>,
    {
        let mut errors = Vec::new();
        for (id, object) in storage.list_mut() {
            if let Err(err) = self.recreate_if_needed(id, object, project) {
                let err = SourcedError::new(id.into(), err);
                log::error!("Error while recreating {id:?}: {:?}", err.error);
                errors.push(err);
            }
        }
        errors
    }

    pub fn happened(&self, object_id: impl Into<ProjectEvent>) -> bool {
        self.events.contains(&object_id.into())
    }
}
