use crate::{
    error::{AppError, AppResult, SourcedError},
    project::{
        ProjectResource, ProjectResourceId,
        storage::{RuntimeStorage, Storage},
    },
};

pub struct RecreateTracker {
    recreations: Vec<ProjectResourceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Revision(usize);

impl Revision {
    pub fn increase(&mut self) {
        self.0 += 1;
    }
}

pub trait Recreatable: ProjectResource {
    type Context<'a>;
    type Runtime;

    fn revision(&self) -> Revision;

    fn needs_rebuild_from_others(&self, tracker: &RecreateTracker) -> bool;

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>>;
}

pub enum SyncOutcome<T> {
    Recreated(T),
    Kept(T),
}

pub enum RuntimeCell<R> {
    Created { runtime: R, revision: Revision },
    Errored { at_revision: Revision },
    Empty,
}

impl RecreateTracker {
    pub fn new() -> Self {
        Self {
            recreations: Vec::new(),
        }
    }

    pub fn ensure<'a, R: Recreatable>(
        &mut self,
        id: R::Id,
        storage: &mut Storage<R>,
        runtime_storage: &'a mut RuntimeStorage<R>,
        ctx: &mut R::Context<'_>,
    ) -> AppResult<&'a R::Runtime>
    where
        R::Id: slotmap::Key + Into<ProjectResourceId>,
    {
        let resource = storage.get(id)?;
        let current_revision = resource.revision();
        let should_rebuild_from_others = resource.needs_rebuild_from_others(self);

        let should_rebuild = match runtime_storage
            .cell_mut(id)
            .ok_or_else(|| AppError::InvalidResource(id.into()))?
        {
            RuntimeCell::Created { revision, .. } => {
                *revision != current_revision || should_rebuild_from_others
            }
            RuntimeCell::Errored { at_revision } => {
                *at_revision != current_revision || should_rebuild_from_others
            }
            RuntimeCell::Empty => true,
        };

        if !should_rebuild {
            return runtime_storage.get(id)?.ok_or(AppError::UninitResource);
        }

        let previous = match std::mem::replace(
            runtime_storage
                .cell_mut(id)
                .ok_or_else(|| AppError::InvalidResource(id.into()))?,
            RuntimeCell::Empty,
        ) {
            RuntimeCell::Created { runtime, .. } => Some(runtime),
            RuntimeCell::Errored { .. } | RuntimeCell::Empty => None,
        };

        match storage.get_mut(id)?.sync(ctx, previous) {
            Ok(SyncOutcome::Recreated(runtime)) => {
                log::info!("Recreated: {:?}", id);
                self.recreations.push(id.into());
                *runtime_storage
                    .cell_mut(id)
                    .ok_or_else(|| AppError::InvalidResource(id.into()))? = RuntimeCell::Created {
                    runtime,
                    revision: current_revision,
                };
            }
            Ok(SyncOutcome::Kept(runtime)) => {
                *runtime_storage
                    .cell_mut(id)
                    .ok_or_else(|| AppError::InvalidResource(id.into()))? = RuntimeCell::Created {
                    runtime,
                    revision: current_revision,
                };
            }
            Err(err) => {
                *runtime_storage
                    .cell_mut(id)
                    .ok_or_else(|| AppError::InvalidResource(id.into()))? = RuntimeCell::Errored {
                    at_revision: current_revision,
                };
                return Err(err);
            }
        }

        runtime_storage.get(id)?.ok_or(AppError::UninitResource)
    }

    fn sync<R: Recreatable>(
        &mut self,
        id: R::Id,
        storage: &mut Storage<R>,
        runtime_storage: &mut RuntimeStorage<R>,
        ctx: &mut R::Context<'_>,
    ) -> Result<(), SourcedError>
    where
        R::Id: slotmap::Key + Into<ProjectResourceId>,
    {
        self.ensure(id, storage, runtime_storage, ctx)
            .map(|_| ())
            .map_err(|error| {
                let err = SourcedError::new(id.into(), error);
                log::error!("Error while syncing {id:?}: {:?}", err.error);
                err
            })
    }

    pub fn sync_storage<'ctx, R: Recreatable>(
        &mut self,
        storage: &mut Storage<R>,
        runtime_storage: &mut RuntimeStorage<R>,
        ctx: &mut R::Context<'ctx>,
    ) -> Vec<SourcedError>
    where
        R::Id: slotmap::Key + Into<ProjectResourceId>,
    {
        let mut errors = Vec::new();
        let ids = storage.list().map(|(id, _)| id).collect::<Vec<_>>();

        for id in ids {
            if let Err(err) = self.sync(id, storage, runtime_storage, ctx) {
                errors.push(err);
            }
        }

        errors
    }

    pub fn was_recreated(&self, object_id: impl Into<ProjectResourceId>) -> bool {
        self.recreations.contains(&object_id.into())
    }
}
