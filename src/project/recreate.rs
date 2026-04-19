use crate::{
    error::{AppError, AppResult, WgpuErrorScope},
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
    Created {
        runtime: R,
        revision: Revision,
    },
    Errored {
        at_revision: Revision,
        error: AppError,
    },
    Empty,
}

impl RecreateTracker {
    pub fn new() -> Self {
        Self {
            recreations: Vec::new(),
        }
    }

    /// Creates the runtime variant of the resource tied with the given id.
    ///
    /// This function will return:
    /// - `Ok(Some(runtime))` if the resource was successfully recreated;
    /// - `Ok(None)` if the resource sync errored;
    /// - `Err(AppError::InvalidResource)` if the resource could not be found;
    pub fn sync<'a, R: Recreatable>(
        &mut self,
        id: R::Id,
        storage: &mut Storage<R>,
        runtime_storage: &'a mut RuntimeStorage<R>,
        ctx: &mut R::Context<'_>,
        device: &wgpu::Device,
    ) -> AppResult<Option<&'a R::Runtime>> {
        let resource = storage.get_mut(id)?;
        let current_revision = resource.revision();
        let should_rebuild_from_others = resource.needs_rebuild_from_others(self);

        let cell = runtime_storage.cell_mut(id)?;

        let should_rebuild = match &*cell {
            RuntimeCell::Created { revision, .. } => {
                *revision != current_revision || should_rebuild_from_others
            }
            RuntimeCell::Errored { at_revision, .. } => {
                *at_revision != current_revision || should_rebuild_from_others
            }
            RuntimeCell::Empty => true,
        };

        if should_rebuild {
            let previous = match std::mem::replace(cell, RuntimeCell::Empty) {
                RuntimeCell::Created { runtime, .. } => Some(runtime),
                RuntimeCell::Errored { .. } | RuntimeCell::Empty => None,
            };

            let scope = WgpuErrorScope::push(device);
            let sync_result = resource.sync(ctx, previous);
            let scope_result = scope.pop();

            match scope_result.and(sync_result) {
                Ok(SyncOutcome::Recreated(runtime)) => {
                    log::debug!("Recreated: {:?}", id);
                    self.recreations.push(id.into());
                    *cell = RuntimeCell::Created {
                        runtime,
                        revision: current_revision,
                    };
                }
                Ok(SyncOutcome::Kept(runtime)) => {
                    *cell = RuntimeCell::Created {
                        runtime,
                        revision: current_revision,
                    };
                }
                Err(err) => {
                    log::error!("Error while syncing {id:?}: {:?}", err);
                    *cell = RuntimeCell::Errored {
                        at_revision: current_revision,
                        error: err,
                    };
                }
            }
        }

        match cell {
            RuntimeCell::Created { runtime, .. } => Ok(Some(runtime)),
            RuntimeCell::Errored { .. } | RuntimeCell::Empty => Ok(None),
        }
    }

    pub fn sync_storage<'ctx, R: Recreatable>(
        &mut self,
        storage: &mut Storage<R>,
        runtime_storage: &mut RuntimeStorage<R>,
        ctx: &mut R::Context<'ctx>,
        device: &wgpu::Device,
    ) {
        let ids = storage.list().map(|(id, _)| id).collect::<Vec<_>>();

        for id in ids {
            let _ = self.sync(id, storage, runtime_storage, ctx, device);
        }
    }

    pub fn was_recreated(&self, object_id: impl Into<ProjectResourceId>) -> bool {
        self.recreations.contains(&object_id.into())
    }
}
