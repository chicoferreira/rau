use slotmap::{SecondaryMap, SlotMap};

use crate::{
    error::{AppError, AppResult},
    project::{
        ProjectResource,
        recreate::{Recreatable, RuntimeCell},
    },
};

pub struct Storage<R: ProjectResource> {
    map: SlotMap<R::Id, R>,
}

impl<R: ProjectResource> Default for Storage<R> {
    fn default() -> Self {
        Self {
            map: SlotMap::default(),
        }
    }
}

impl<R: ProjectResource> Storage<R> {
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

pub struct RuntimeStorage<R: Recreatable> {
    map: SecondaryMap<R::Id, RuntimeCell<R::Runtime>>,
}

impl<R: Recreatable> Default for RuntimeStorage<R> {
    fn default() -> Self {
        Self {
            map: SecondaryMap::default(),
        }
    }
}

impl<R: Recreatable> RuntimeStorage<R> {
    pub fn get_cell(&self, key: R::Id) -> AppResult<&RuntimeCell<R::Runtime>> {
        self.map
            .get(key)
            .ok_or_else(|| AppError::InvalidResource(key.into()))
    }

    pub fn get_cell_mut(&mut self, key: R::Id) -> AppResult<&mut RuntimeCell<R::Runtime>> {
        Ok(self
            .map
            .entry(key)
            .ok_or_else(|| AppError::InvalidResource(key.into()))?
            .or_insert(RuntimeCell::Empty))
    }

    pub fn get(&self, key: R::Id) -> AppResult<&R::Runtime> {
        self.get_cell(key).and_then(|cell| match cell {
            RuntimeCell::Created { runtime, .. } => Ok(runtime),
            RuntimeCell::Errored | RuntimeCell::Empty => Err(AppError::InvalidResource(key.into())), // TODO: this is wrong, if it is empty we should return another thing
        })
    }
}
