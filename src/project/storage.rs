use slotmap::SlotMap;

pub struct Storage<Key: slotmap::Key, Value> {
    map: SlotMap<Key, Value>,
}

impl<Key: slotmap::Key, Value> Storage<Key, Value> {
    pub fn new() -> Self {
        Self {
            map: SlotMap::default(),
        }
    }

    pub fn get(&self, key: Key) -> Option<&Value> {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut Value> {
        self.map.get_mut(key)
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
