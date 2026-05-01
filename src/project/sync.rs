use crate::{
    error::{AppError, AppResult},
    project::{
        ProjectResource, ResourceId,
        file::ProjectFilePath,
        storage::{RuntimeStorage, Storage},
    },
};

#[derive(Default)]
pub struct SyncTracker {
    resource_changes: Vec<ResourceId>,
    file_changes: Vec<ProjectFilePath>,
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
    type Job: Default;

    fn revision(&self) -> Revision;

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool;

    fn should_sync(
        &self,
        tracker: &SyncTracker,
        runtime: &RuntimeCell<Self::Runtime, Self::Job>,
    ) -> bool {
        let current_revision = self.revision();
        let should_rebuild_from_others = self.needs_rebuild_from_others(tracker);

        let should_rebuild = match runtime {
            RuntimeCell::Created { revision, .. } => *revision != current_revision,
            RuntimeCell::Errored {
                revision: at_revision,
                ..
            } => *at_revision != current_revision,
            RuntimeCell::Pending {
                revision: at_revision,
                ..
            } => *at_revision != current_revision,
            RuntimeCell::Empty => true,
        };

        should_rebuild || should_rebuild_from_others
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>>;

    // TODO: remove this once we separate the RenderPipeline from RenderPass.
    fn after_sync(&mut self) {}
}

pub enum SyncOutcome<R, P> {
    Changed(R),
    Unchanged(R),
    Pending(P),
}

#[derive(Debug, Default)]
pub enum RuntimeCell<R, P> {
    Created {
        runtime: R,
        revision: Revision,
    },
    Errored {
        error: AppError,
        revision: Revision,
    },
    Pending {
        job: P,
        revision: Revision,
    },
    #[default]
    Empty,
}

impl<R, P> RuntimeCell<R, P> {
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn get_error(&self, id: impl Into<ResourceId>) -> Option<(ResourceId, &AppError)> {
        if let RuntimeCell::Errored { error, .. } = self {
            Some((id.into(), error))
        } else {
            None
        }
    }
}

impl SyncTracker {
    pub fn clear_changes(&mut self) {
        self.resource_changes.clear();
        self.file_changes.clear();
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
    ) -> AppResult<Option<&'a R::Runtime>>
    where
        R::Id: slotmap::Key,
    {
        let resource = storage.get_mut(id)?;
        let cell = runtime_storage.cell_mut(id)?;
        self.sync_singleton(id, resource, cell, ctx)
    }

    pub fn sync_storage<'ctx, R: SyncResource>(
        &mut self,
        storage: &mut Storage<R>,
        runtime_storage: &mut RuntimeStorage<R>,
        ctx: &mut R::Context<'ctx>,
    ) where
        R::Id: slotmap::Key,
    {
        let ids = storage.list().map(|(id, _)| id).collect::<Vec<_>>();

        for id in ids {
            let _ = self.sync(id, storage, runtime_storage, ctx);
        }
    }

    pub fn sync_singleton<'a, R: SyncResource>(
        &mut self,
        id: R::Id,
        resource: &mut R,
        cell: &'a mut RuntimeCell<R::Runtime, R::Job>,
        ctx: &mut R::Context<'_>,
    ) -> AppResult<Option<&'a R::Runtime>> {
        let current_revision = resource.revision();
        let id = id.into();

        if resource.should_sync(self, cell) {
            let previous = match cell.take() {
                RuntimeCell::Created { runtime, .. } => Some(runtime),
                RuntimeCell::Errored { .. } | RuntimeCell::Pending { .. } | RuntimeCell::Empty => {
                    None
                }
            };

            let sync_result = resource.sync(ctx, previous, R::Job::default());
            self.apply_sync_result(id, resource, cell, current_revision, sync_result);
        } else if matches!(cell, RuntimeCell::Pending { .. }) {
            let RuntimeCell::Pending { job, .. } = cell.take() else {
                unreachable!();
            };

            let sync_result = resource.sync(ctx, None, job);
            self.apply_sync_result(id, resource, cell, current_revision, sync_result);
        }

        match cell {
            RuntimeCell::Created { runtime, .. } => Ok(Some(runtime)),
            RuntimeCell::Errored { .. } | RuntimeCell::Pending { .. } | RuntimeCell::Empty => {
                Ok(None)
            }
        }
    }

    fn apply_sync_result<R: SyncResource>(
        &mut self,
        id: ResourceId,
        resource: &mut R,
        cell: &mut RuntimeCell<R::Runtime, R::Job>,
        revision: Revision,
        sync_result: AppResult<SyncOutcome<R::Runtime, R::Job>>,
    ) {
        match sync_result {
            Ok(SyncOutcome::Changed(runtime)) => {
                *cell = RuntimeCell::Created { runtime, revision };
                resource.after_sync();
                log::debug!("Recreated: {:?}", id);
                self.resource_changes.push(id);
            }
            Ok(SyncOutcome::Unchanged(runtime)) => {
                *cell = RuntimeCell::Created { runtime, revision };
                resource.after_sync();
            }
            Ok(SyncOutcome::Pending(job)) => {
                *cell = RuntimeCell::Pending { job, revision };
            }
            Err(err) => {
                log::error!("Error while syncing {id:?}: {:?}", err);
                self.resource_changes.push(id);
                *cell = RuntimeCell::Errored {
                    revision,
                    error: err,
                };
            }
        }
    }

    pub fn was_changed(&self, object_id: impl Into<ResourceId>) -> bool {
        self.resource_changes.contains(&object_id.into())
    }

    pub fn has_resource_changes(&self) -> bool {
        !self.resource_changes.is_empty()
    }

    pub fn file_changed(&self, path: &ProjectFilePath) -> bool {
        self.file_changes.contains(path)
    }

    pub(crate) fn push_resource_change(&mut self, id: ResourceId) {
        self.resource_changes.push(id);
    }

    pub(crate) fn push_file_changes(&mut self, paths: Vec<ProjectFilePath>) {
        self.file_changes.extend(paths);
    }
}
