use crate::{
    app::{AppEvent, State},
    main_menu::MainMenu,
    project::{ResourceId, paths::FilePath},
    ui::rename::RenameTarget,
    workspace::StateEvent,
};

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

    pub fn add_all(&mut self, events: &mut Vec<T>) {
        self.events.append(events);
    }

    pub fn drain(&mut self) -> std::vec::Drain<'_, T> {
        self.events.drain(..)
    }
}

impl EventQueue<StateEvent> {
    pub fn inspect_resource(&mut self, id: impl Into<ResourceId>) {
        self.add(StateEvent::InspectResource(id.into()));
    }

    pub fn open_file(&mut self, path: FilePath) {
        self.add(StateEvent::OpenFile(path));
    }

    pub fn start_rename(&mut self, target: RenameTarget) {
        self.add(StateEvent::StartRename(target));
    }

    pub fn apply_rename(&mut self, target: RenameTarget, label: String) {
        self.add(StateEvent::ApplyRename(target, label));
    }

    pub fn cancel_rename(&mut self) {
        self.add(StateEvent::CancelRename);
    }
}

impl EventQueue<AppEvent> {
    pub fn close_project(&mut self) {
        self.add(AppEvent::SetState(State::MainMenu(MainMenu::default())));
    }
}
