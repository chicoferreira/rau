use slotmap::SlotMap;

use crate::{
    error::{AppError, AppResult},
    project::{ProjectResource, ProjectResourceId},
};

pub struct Storage<Key: slotmap::Key, Value> {
    map: SlotMap<Key, Value>,
}

impl<Key: slotmap::Key, Value> Storage<Key, Value> {
    pub fn new() -> Self {
        Self {
            map: SlotMap::default(),
        }
    }

    pub fn list(&self) -> impl Iterator<Item = (Key, &Value)> {
        self.map.iter()
    }

    pub fn list_mut(&mut self) -> impl Iterator<Item = (Key, &mut Value)> {
        self.map.iter_mut()
    }

    pub fn register(&mut self, value: Value) -> Key {
        self.map.insert(value)
    }

    pub fn unregister(&mut self, key: Key) -> Option<Value> {
        self.map.remove(key)
    }
}

impl<Key: slotmap::Key + Into<ProjectResourceId>, Value> Storage<Key, Value> {
    pub fn get(&self, key: Key) -> AppResult<&Value> {
        self.map
            .get(key)
            .ok_or_else(|| AppError::InvalidResource(key.into()))
    }

    pub fn get_mut(&mut self, key: Key) -> AppResult<&mut Value> {
        self.map
            .get_mut(key)
            .ok_or_else(|| AppError::InvalidResource(key.into()))
    }
}

impl<Key: slotmap::Key + Into<ProjectResourceId>, Value: ProjectResource> Storage<Key, Value> {
    pub fn get_label(&self, key: Key) -> AppResult<&str> {
        Ok(self.get(key)?.label())
    }
}
