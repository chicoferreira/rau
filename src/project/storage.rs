use itertools::Itertools;
use serde::{Deserialize, Serialize};
use slotmap::{SecondaryMap, SlotMap};

use crate::{
    error::{AppError, AppResult},
    project::{
        Creatable, ProjectResource, ResourceId,
        sync::{Revision, RuntimeCell, SyncResource},
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
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn list_sorted(&self) -> impl Iterator<Item = (R::Id, &R)> + '_ {
        self.map
            .iter()
            .sorted_by_key(|(_, res)| res.label().to_lowercase())
    }

    pub fn list(&self) -> impl Iterator<Item = (R::Id, &R)> {
        self.map.iter()
    }

    pub fn list_mut(&mut self) -> impl Iterator<Item = (R::Id, &mut R)> {
        self.map.iter_mut()
    }

    pub fn project_revisions(&self) -> impl Iterator<Item = (ResourceId, Revision)> + '_ {
        self.list()
            .map(|(id, resource)| (id.into(), resource.project_revision()))
    }

    pub fn register(&mut self, value: R) -> R::Id {
        self.map.insert(value)
    }

    pub fn has_label(&self, label: &str) -> bool {
        self.map.values().any(|resource| resource.label() == label)
    }

    pub fn next_label(&self, preferred_label: &str) -> String {
        if !self.has_label(preferred_label) {
            return preferred_label.to_owned();
        }

        let mut index = 1;
        loop {
            let label = format!("{preferred_label} ({index})");
            if !self.has_label(&label) {
                return label;
            }
            index += 1;
        }
    }

    pub fn unregister(&mut self, id: R::Id) {
        self.map.remove(id);
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

impl<R> Storage<R>
where
    R: Creatable,
    R::Id: slotmap::Key,
{
    pub fn create(&mut self, label: String) -> R::Id {
        self.register(R::create(label))
    }
}

impl<R> Serialize for Storage<R>
where
    R: ProjectResource + Serialize,
    R::Id: slotmap::Key,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.map.serialize(serializer)
    }
}

impl<'a, R> Deserialize<'a> for Storage<R>
where
    R: ProjectResource + Deserialize<'a>,
    R::Id: slotmap::Key,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(Self {
            map: SlotMap::deserialize(deserializer)?,
        })
    }
}

pub struct RuntimeStorage<R>
where
    R: SyncResource,
    R::Id: slotmap::Key,
{
    map: SecondaryMap<R::Id, RuntimeCell<R::Runtime, R::Job>>,
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
    pub fn unregister(&mut self, key: R::Id) {
        self.map.remove(key);
    }

    pub fn get_init(&self, key: R::Id) -> AppResult<Option<&R::Runtime>> {
        let id = key.into();
        match self.map.get(key) {
            Some(RuntimeCell::Created { runtime, .. }) => Ok(Some(runtime)),
            Some(RuntimeCell::Pending { .. }) => Ok(None),
            Some(RuntimeCell::Empty) => Ok(None),
            Some(RuntimeCell::Errored { .. }) => Err(AppError::WaitingForErroredResource(id)),
            None => Err(AppError::InvalidResource(id)),
        }
    }

    /// Returns a mutable reference to the [`RuntimeCell`] for the given key.
    /// Returns `AppError::InvalidResource` if the key is not found.
    pub(super) fn cell_mut(
        &mut self,
        key: R::Id,
    ) -> AppResult<&mut RuntimeCell<R::Runtime, R::Job>> {
        self.map
            .entry(key)
            .map(|entry| entry.or_insert(RuntimeCell::Empty))
            .ok_or_else(|| AppError::InvalidResource(key.into()))
    }

    pub fn has_pending(&self) -> bool {
        self.map
            .values()
            .any(|cell| matches!(cell, RuntimeCell::Pending { .. }))
    }

    pub fn mark_errored(&mut self, key: R::Id, error: AppError) {
        let Ok(cell) = self.cell_mut(key) else {
            return;
        };
        let revision = match cell {
            RuntimeCell::Created { revision, .. }
            | RuntimeCell::Errored { revision, .. }
            | RuntimeCell::Pending { revision, .. } => *revision,
            RuntimeCell::Empty => Revision::default(),
        };
        *cell = RuntimeCell::Errored { error, revision };
    }

    pub fn get_error(&self, key: R::Id) -> Option<&AppError> {
        match self.map.get(key) {
            Some(RuntimeCell::Errored { error, .. }) => Some(error),
            _ => None,
        }
    }

    pub fn get_errors(&self) -> impl Iterator<Item = (ResourceId, &AppError)> {
        self.map.iter().filter_map(|(key, cell)| {
            if let RuntimeCell::Errored { error, .. } = cell {
                Some((key.into(), error))
            } else {
                None
            }
        })
    }
}
