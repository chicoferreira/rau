use crate::{
    error::{AppError, AppResult},
    project::{
        ProjectResource, ResourceId,
        paths::FilePath,
        storage::{RuntimeStorage, Storage},
    },
};

/// Per-frame change tracking for runtime resources.
#[derive(Default)]
pub struct SyncTracker {
    /// Resources whose runtime object's identity changed this frame (a new wgpu
    /// object was created). Resources *built from* one of them (e.g. a bind
    /// group holding the previous buffer) must rebuild.
    recreated: Vec<ResourceId>,
    /// Resources whose observable data changed this frame, with or without
    /// recreation (e.g. a uniform buffer rewritten in place). Resources that
    /// *execute over* the data (e.g. a compute pass) must rerun, while holders
    /// of the runtime object don't need to rebuild. Recreation implies a data
    /// change, so this is a superset of `recreated`.
    data_changes: Vec<ResourceId>,
    /// Files whose contents changed this frame, so file-backed resources
    /// (shaders, textures, models) can resync.
    file_changes: Vec<FilePath>,
}

/// Monotonic counter bumped whenever a resource is mutated; comparing it against
/// the revision stored in a [`RuntimeCell`] tells whether the runtime is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Revision(usize);

impl Revision {
    pub fn increase(&mut self) {
        self.0 += 1;
    }
}

/// A project resource that can be materialized into a runtime (usually wgpu)
/// counterpart and kept in sync with it frame by frame.
pub trait SyncResource: ProjectResource {
    /// External state needed to build the runtime (device, queue, other storages…).
    type Context<'a>;
    /// The materialized counterpart of this resource.
    type Runtime;
    /// State carried across frames while a sync is in flight; `Default` is the
    /// initial "start from scratch" job.
    type Job: Default;

    /// Revision of the fields the runtime is built from; a mismatch with the
    /// stored cell revision triggers a resync.
    fn runtime_revision(&self) -> Revision;

    /// Whether this resource must be resynced because something it is built from
    /// changed this frame (beyond its own `runtime_revision`).
    ///
    /// Resources holding references to a dependency's runtime object should check
    /// [`SyncTracker::was_recreated`]; resources consuming a dependency's data
    /// should check [`SyncTracker::was_data_changed`].
    fn needs_rebuild(&self, id: Self::Id, ctx: &Self::Context<'_>, tracker: &SyncTracker) -> bool;

    /// Whether data observable through this resource changed this frame even though
    /// its own runtime didn't need a rebuild. When true, the tracker records the resource
    /// as data-changed so consumers further down (such as compute passes) rerun.
    fn forwards_data_changes(&self, _: Self::Id, _: &Self::Context<'_>, _: &SyncTracker) -> bool {
        false
    }

    /// Builds or updates the runtime. `previous` is the current runtime (if any)
    /// so it can be reused, and `job` resumes any in-flight work from last frame.
    fn sync<'a>(
        &self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>>;
}

/// What a [`SyncResource::sync`] call did to the runtime.
#[derive(Clone)]
pub enum SyncOutcome<R, P> {
    /// The runtime object was recreated. Dependents holding references to the
    /// previous object must rebuild. Implies the observable data changed too.
    Recreated(R),
    /// The runtime object kept its identity but the data observable through it
    /// changed (e.g. a buffer rewritten in place). Consumers must re-execute;
    /// holders of the object don't need to rebuild.
    DataChanged(R),
    /// Nothing observable changed.
    Unchanged(R),
    /// The sync is still in progress; the job is stored and resumed next frame.
    Pending(P),
}

/// Per-resource slot holding the runtime state and the revision it was last
/// synced at.
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
    /// Never synced yet.
    #[default]
    Empty,
}

impl<R, P> RuntimeCell<R, P> {
    /// Takes the cell's contents, leaving [`RuntimeCell::Empty`] behind.
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl SyncTracker {
    /// Forgets all changes recorded this frame; called once per frame after
    /// every storage has been synced.
    pub fn clear_changes(&mut self) {
        self.recreated.clear();
        self.data_changes.clear();
        self.file_changes.clear();
    }

    /// Syncs the resource tied with the given id, looking it up in its storages.
    ///
    /// This function will return:
    /// - `Ok(Some(runtime))` if the resource has an up-to-date runtime;
    /// - `Ok(None)` if the resource sync errored or is still pending;
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

    /// Syncs every resource in the storage, ignoring individual failures
    /// (they are recorded in each resource's [`RuntimeCell`]).
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

    /// Whether the resource must resync: its own revision moved past the cell's,
    /// or a dependency changed ([`SyncResource::needs_rebuild`]).
    fn should_sync<R: SyncResource>(
        &self,
        id: R::Id,
        resource: &R,
        cell: &RuntimeCell<R::Runtime, R::Job>,
        ctx: &R::Context<'_>,
    ) -> bool {
        let current_revision = resource.runtime_revision();
        let should_rebuild_from_others = resource.needs_rebuild(id, ctx, self);

        let should_rebuild = match cell {
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

    /// Syncs one resource against its [`RuntimeCell`]: advances or starts a sync
    /// when needed, records the outcome, and returns the runtime if available.
    pub fn sync_singleton<'a, R: SyncResource>(
        &mut self,
        id: R::Id,
        resource: &mut R,
        cell: &'a mut RuntimeCell<R::Runtime, R::Job>,
        ctx: &mut R::Context<'_>,
    ) -> AppResult<Option<&'a R::Runtime>> {
        let current_revision = resource.runtime_revision();

        // An in-flight job whose resource is unchanged must be advanced, not restarted: on WASM,
        // WGPU validation futures need to yield to the browser event loop, so an always-changing
        // resource such as a time uniform would otherwise stay pending forever. A revision change
        // makes the job obsolete, so let those fall through to `should_sync` and rebuild instead.
        let advance_pending = matches!(
            cell,
            RuntimeCell::Pending { revision, .. } if *revision == current_revision
        );

        let action = if advance_pending {
            let RuntimeCell::Pending { job, .. } = cell.take() else {
                unreachable!();
            };
            Some((None, job))
        } else if self.should_sync(id, resource, cell, ctx) {
            let previous = match cell.take() {
                RuntimeCell::Created { runtime, .. } => Some(runtime),
                RuntimeCell::Errored { .. } | RuntimeCell::Pending { .. } | RuntimeCell::Empty => {
                    None
                }
            };
            Some((previous, R::Job::default()))
        } else {
            None
        };

        if let Some((previous, job)) = action {
            let sync_result = resource.sync(id, ctx, previous, job);
            self.apply_sync_result::<R>(id, cell, current_revision, sync_result);
        }

        // A resource that didn't rebuild can still expose new data through its
        // unchanged runtime (e.g. a bind group over a uniform buffer that was
        // rewritten in place); record it so consumers downstream rerun.
        if !self.was_data_changed(id) && resource.forwards_data_changes(id, ctx, self) {
            self.data_changes.push(id.into());
        }

        match cell {
            RuntimeCell::Created { runtime, .. } => Ok(Some(runtime)),
            RuntimeCell::Errored { .. } | RuntimeCell::Pending { .. } | RuntimeCell::Empty => {
                Ok(None)
            }
        }
    }

    /// Stores the sync result in the cell and records the corresponding changes.
    fn apply_sync_result<R: SyncResource>(
        &mut self,
        id: R::Id,
        cell: &mut RuntimeCell<R::Runtime, R::Job>,
        revision: Revision,
        sync_result: AppResult<SyncOutcome<R::Runtime, R::Job>>,
    ) {
        match sync_result {
            Ok(SyncOutcome::Recreated(runtime)) => {
                *cell = RuntimeCell::Created { runtime, revision };
                log::debug!("Recreated: {:?}", id);
                self.push_resource_change(id.into());
            }
            Ok(SyncOutcome::DataChanged(runtime)) => {
                *cell = RuntimeCell::Created { runtime, revision };
                self.data_changes.push(id.into());
            }
            Ok(SyncOutcome::Unchanged(runtime)) => {
                *cell = RuntimeCell::Created { runtime, revision };
            }
            Ok(SyncOutcome::Pending(job)) => {
                *cell = RuntimeCell::Pending { job, revision };
            }
            Err(err) => {
                log::error!("Error while syncing {id:?}: {:?}", err);
                self.push_resource_change(id.into());
                *cell = RuntimeCell::Errored {
                    revision,
                    error: err,
                };
            }
        }
    }

    /// Whether the resource's runtime object was recreated this frame.
    pub fn was_recreated(&self, object_id: impl Into<ResourceId>) -> bool {
        self.recreated.contains(&object_id.into())
    }

    /// Whether the data observable through the resource changed this frame.
    /// Includes recreations.
    pub fn was_data_changed(&self, object_id: impl Into<ResourceId>) -> bool {
        self.data_changes.contains(&object_id.into())
    }

    /// Whether any resource changed this frame, in either channel.
    pub fn has_resource_changes(&self) -> bool {
        !self.data_changes.is_empty()
    }

    /// Whether the file's contents changed this frame.
    pub fn file_changed(&self, path: &FilePath) -> bool {
        self.file_changes.contains(path)
    }

    /// Marks the resource as changed in both channels, e.g. when it is deleted
    /// outside the sync loop.
    pub(crate) fn push_resource_change(&mut self, id: ResourceId) {
        self.recreated.push(id);
        self.data_changes.push(id);
    }

    pub(crate) fn push_file_changes(&mut self, paths: impl IntoIterator<Item = FilePath>) {
        self.file_changes.extend(paths);
    }
}
