#[derive(Debug)]
pub struct EventQueue<T> {
    events: Vec<T>,
}

impl<T> Default for EventQueue<T> {
    fn default() -> Self {
        Self { events: Vec::new() }
    }
}

impl<T> EventQueue<T> {
    pub fn add(&mut self, event: T) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, T> {
        self.events.drain(..)
    }
}
