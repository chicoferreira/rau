pub trait VecSetAtExtension {
    type Item;

    fn set_at(&mut self, index: usize, value: Self::Item);
}

impl<T: Clone + Default> VecSetAtExtension for Vec<T> {
    type Item = T;

    fn set_at(&mut self, index: usize, value: T) {
        if index >= self.len() {
            self.resize(index + 1, Default::default());
        }
        self[index] = value;
    }
}
