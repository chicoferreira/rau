use crate::{
    error::{AppError, AppResult},
    project::{
        ProjectResource, ResourceId,
        storage::{RuntimeStorage, Storage},
    },
    utils::wgpu_error_scope::{WgpuErrorScope, WgpuErrorScopeWaiter},
};

#[derive(Default)]
pub struct SyncTracker {
    changes: Vec<ResourceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Revision(usize);

impl Revision {
    pub fn increase(&mut self) {
        self.0 += 1;
    }
}

pub trait SyncResource: ProjectResource {
    type Context<'a>;
    type Runtime;

    fn revision(&self) -> Revision;

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool;

    fn should_sync(&self, tracker: &SyncTracker, runtime: &RuntimeCell<Self::Runtime>) -> bool {
        let current_revision = self.revision();
        let should_rebuild_from_others = self.needs_rebuild_from_others(tracker);

        let should_rebuild = match runtime {
            RuntimeCell::Created { revision, .. } => *revision != current_revision,
            RuntimeCell::Errored {
                revision: at_revision,
                ..
            } => *at_revision != current_revision,
            RuntimeCell::PendingValidation {
                revision: at_revision,
                ..
            } => *at_revision != current_revision,
            RuntimeCell::Empty => true,
        };

        should_rebuild || should_rebuild_from_others
    }

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>>;
}

pub enum SyncOutcome<T> {
    Changed(T),
    Unchanged(T),
}

#[derive(Debug, Default)]
pub enum RuntimeCell<R> {
    Created {
        runtime: R,
        revision: Revision,
    },
    Errored {
        error: AppError,
        revision: Revision,
    },
    PendingValidation {
        runtime: R,
        revision: Revision,
    },
    #[default]
    Empty,
}

impl<R> RuntimeCell<R> {
    pub fn get_error(&self, id: impl Into<ResourceId>) -> Option<(ResourceId, &AppError)> {
        if let RuntimeCell::Errored { error, .. } = self {
            Some((id.into(), error))
        } else {
            None
        }
    }

    pub fn handle_validation(&mut self, rev: Revision, error: Option<wgpu::Error>) {
        let RuntimeCell::PendingValidation { revision, .. } = self else {
            return;
        };

        if *revision != rev {
            return;
        }

        let RuntimeCell::PendingValidation { runtime, revision } =
            std::mem::replace(self, RuntimeCell::Empty)
        else {
            unreachable!();
        };

        *self = match error {
            Some(error) => RuntimeCell::Errored {
                revision,
                error: error.into(),
            },
            None => RuntimeCell::Created { runtime, revision },
        };
    }
}

impl SyncTracker {
    pub fn clear_changes(&mut self) {
        self.changes.clear();
    }

    /// Creates the runtime variant of the resource tied with the given id.
    ///
    /// This function will return:
    /// - `Ok(Some(runtime))` if the resource was successfully recreated;
    /// - `Ok(None)` if the resource sync errored;
    /// - `Err(AppError::InvalidResource)` if the resource could not be found;
    pub fn sync<'a, R: SyncResource>(
        &mut self,
        id: R::Id,
        storage: &mut Storage<R>,
        runtime_storage: &'a mut RuntimeStorage<R>,
        ctx: &mut R::Context<'_>,
        device: &wgpu::Device,
        error_waiter: &WgpuErrorScopeWaiter,
    ) -> AppResult<Option<&'a R::Runtime>>
    where
        R::Id: slotmap::Key,
    {
        let resource = storage.get_mut(id)?;
        let cell = runtime_storage.cell_mut(id)?;
        self.sync_singleton(id, resource, cell, ctx, device, error_waiter)
    }

    pub fn sync_storage<'ctx, R: SyncResource>(
        &mut self,
        storage: &mut Storage<R>,
        runtime_storage: &mut RuntimeStorage<R>,
        ctx: &mut R::Context<'ctx>,
        device: &wgpu::Device,
        error_waiter: &WgpuErrorScopeWaiter,
    ) where
        R::Id: slotmap::Key,
    {
        let ids = storage.list().map(|(id, _)| id).collect::<Vec<_>>();

        for id in ids {
            let _ = self.sync(id, storage, runtime_storage, ctx, device, error_waiter);
        }
    }

    pub fn sync_singleton<'a, R: SyncResource>(
        &mut self,
        id: R::Id,
        resource: &mut R,
        cell: &'a mut RuntimeCell<R::Runtime>,
        ctx: &mut R::Context<'_>,
        device: &wgpu::Device,
        error_waiter: &WgpuErrorScopeWaiter,
    ) -> AppResult<Option<&'a R::Runtime>> {
        let current_revision = resource.revision();

        if resource.should_sync(self, cell) {
            let previous = match std::mem::replace(cell, RuntimeCell::Empty) {
                RuntimeCell::Created { runtime, .. } => Some(runtime),
                RuntimeCell::Errored { .. }
                | RuntimeCell::PendingValidation { .. }
                | RuntimeCell::Empty => None,
            };

            let scope = WgpuErrorScope::push(device);
            let sync_result = resource.sync(ctx, previous);

            let id = id.into();
            match sync_result {
                Ok(SyncOutcome::Changed(runtime)) => {
                    log::debug!("Recreated: {:?}", id);
                    self.changes.push(id.into());
                    *cell = RuntimeCell::PendingValidation {
                        runtime,
                        revision: current_revision,
                    };
                    // On the other variants the `Drop´ impl for the error scope will already pop the error by itself.
                    error_waiter.pop_error(id, current_revision, scope);
                }
                Ok(SyncOutcome::Unchanged(runtime)) => {
                    *cell = RuntimeCell::Created {
                        runtime,
                        revision: current_revision,
                    };
                }
                Err(err) => {
                    log::error!("Error while syncing {id:?}: {:?}", err);
                    self.changes.push(id.into());
                    *cell = RuntimeCell::Errored {
                        revision: current_revision,
                        error: err,
                    };
                }
            }
        }

        match cell {
            RuntimeCell::Created { runtime, .. } => Ok(Some(runtime)),
            RuntimeCell::Errored { .. }
            | RuntimeCell::PendingValidation { .. }
            | RuntimeCell::Empty => Ok(None),
        }
    }

    pub fn was_changed(&self, object_id: impl Into<ResourceId>) -> bool {
        self.changes.contains(&object_id.into())
    }

    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub(crate) fn push_change(&mut self, id: ResourceId) {
        self.changes.push(id);
    }
}
