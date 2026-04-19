use std::mem;

use crate::{
    error::{AppResult, SourcedError},
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
        runtime: &mut Option<Self::Runtime>,
    ) -> AppResult<SyncResult>;
}

pub enum SyncResult {
    Recreated,
    Nothing,
}

pub enum RuntimeCell<R> {
    Created { runtime: R, revision: Revision },
    Errored,
    Empty,
}

impl RecreateTracker {
    pub fn new() -> Self {
        Self {
            recreations: Vec::new(),
        }
    }

    pub fn sync<'ctx, R: Recreatable>(
        &mut self,
        id: R::Id,
        resource: &mut R,
        runtime_resource: &mut RuntimeCell<R::Runtime>,
        ctx: &mut R::Context<'ctx>,
    ) -> Result<(), SourcedError>
    where
        R::Id: slotmap::Key + Into<ProjectResourceId>,
    {
        let current_revision = resource.revision();

        let should_rebuild_from_itself = match runtime_resource {
            RuntimeCell::Created { revision, .. } => *revision != current_revision,
            RuntimeCell::Errored { .. } => true, // change this to only rebuild if something happened
            RuntimeCell::Empty => true,
        };

        let should_rebuild_from_others = resource.needs_rebuild_from_others(&self);

        if should_rebuild_from_itself || should_rebuild_from_others {
            let previous_cell = mem::replace(runtime_resource, RuntimeCell::Empty);
            let mut runtime = match previous_cell {
                RuntimeCell::Created { runtime, .. } => Some(runtime),
                _ => None,
            };

            match resource.sync(ctx, &mut runtime) {
                Ok(result) => {
                    match result {
                        SyncResult::Recreated => {
                            log::debug!("Recreated: {:?}", id);
                            self.recreations.push(id.into());
                        }
                        SyncResult::Nothing => {}
                    };

                    *runtime_resource = match runtime {
                        Some(runtime) => RuntimeCell::Created {
                            runtime,
                            revision: current_revision,
                        },
                        None => RuntimeCell::Empty,
                    };
                }
                Err(err) => {
                    let err = SourcedError::new(id.into(), err);
                    log::error!("Error while syncing {id:?}: {:?}", err.error);

                    *runtime_resource = RuntimeCell::Errored;

                    return Err(err);
                }
            }
        }
        return Ok(());
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
        for (id, object) in storage.list_mut() {
            let Ok(cell) = runtime_storage.get_cell_mut(id) else {
                continue;
            };

            if let Err(err) = self.sync(id, object, cell, ctx) {
                errors.push(err);
            }
        }
        errors
    }

    pub fn was_recreated(&self, object_id: impl Into<ProjectResourceId>) -> bool {
        self.recreations.contains(&object_id.into())
    }
}
