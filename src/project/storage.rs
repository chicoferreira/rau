use slotmap::{SecondaryMap, SlotMap};

use crate::{
    error::{AppError, AppResult},
    project::{
        ProjectResource, ProjectResourceId,
        sync::{RuntimeCell, SyncResource},
    },
};

pub struct Storage<R>
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    map: SlotMap<R::Id, R>,
}

impl<R> Default for Storage<R>
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    fn default() -> Self {
        Self {
            map: SlotMap::default(),
        }
    }
}

impl<R> Storage<R>
where
    R: ProjectResource,
    R::Id: slotmap::Key,
{
    pub fn list(&self) -> impl Iterator<Item = (R::Id, &R)> {
        self.map.iter()
    }

    pub fn list_mut(&mut self) -> impl Iterator<Item = (R::Id, &mut R)> {
        self.map.iter_mut()
    }

    pub fn register(&mut self, value: R) -> R::Id {
        self.map.insert(value)
    }

    pub fn unregister(&mut self, id: R::Id) -> Option<R> {
        self.map.remove(id)
    }

    pub fn get(&self, id: R::Id) -> AppResult<&R> {
        self.map
            .get(id)
            .ok_or_else(|| AppError::InvalidResource(id.into()))
    }

    pub fn get_mut(&mut self, id: R::Id) -> AppResult<&mut R> {
        self.map
            .get_mut(id)
            .ok_or_else(|| AppError::InvalidResource(id.into()))
    }

    pub fn get_label(&self, id: R::Id) -> AppResult<&str> {
        Ok(self.get(id)?.label())
    }
}

pub struct RuntimeStorage<R>
where
    R: SyncResource,
    R::Id: slotmap::Key,
{
    map: SecondaryMap<R::Id, RuntimeCell<R::Runtime>>,
}

impl<R> Default for RuntimeStorage<R>
where
    R: SyncResource,
    R::Id: slotmap::Key,
{
    fn default() -> Self {
        Self {
            map: SecondaryMap::default(),
        }
    }
}

impl<R> RuntimeStorage<R>
where
    R: SyncResource,
    R::Id: slotmap::Key,
{
    /// Returns a reference to the [`RuntimeCell`] for the given key.
    /// Returns `AppError::InvalidResource` if the key is not found.
    /// Returns `None` if the runtime value is errored or empty.
    pub fn get(&self, key: R::Id) -> AppResult<Option<&R::Runtime>> {
        match self.map.get(key) {
            Some(RuntimeCell::Created { runtime, .. }) => Ok(Some(runtime)),
            Some(RuntimeCell::Errored { .. } | RuntimeCell::Empty) => Ok(None),
            None => Err(AppError::InvalidResource(key.into())),
        }
    }

    /// Returns a mutable reference to the [`RuntimeCell`] for the given key.
    /// Returns `AppError::InvalidResource` if the key is not found.
    pub(super) fn cell_mut(&mut self, key: R::Id) -> AppResult<&mut RuntimeCell<R::Runtime>> {
        self.map
            .entry(key)
            .map(|entry| entry.or_insert(RuntimeCell::Empty))
            .ok_or_else(|| AppError::InvalidResource(key.into()))
    }

    pub fn get_errors(&self) -> impl Iterator<Item = (ProjectResourceId, &AppError)> {
        self.map.iter().filter_map(|(key, cell)| {
            if let RuntimeCell::Errored { error, .. } = cell {
                Some((key.into(), error))
            } else {
                None
            }
        })
    }
}
