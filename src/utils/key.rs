use enumset::{EnumSet, EnumSetType};

#[derive(Debug, EnumSetType)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Space,
    Shift,
    W,
    A,
    S,
    D,
    V,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardState {
    /// Keys currently held down this frame.
    pressed: EnumSet<Key>,
    /// Keys that went down this frame.
    just_pressed: EnumSet<Key>,
}

impl KeyboardState {
    pub fn empty() -> Self {
        Self {
            pressed: EnumSet::empty(),
            just_pressed: EnumSet::empty(),
        }
    }

    pub fn from_egui_input(input: &egui::InputState) -> Self {
        let mut pressed: EnumSet<Key> = input
            .keys_down
            .iter()
            .filter_map(|k| Key::try_from(k).ok())
            .collect();

        if input.modifiers.shift {
            pressed.insert(Key::Shift);
        }

        let just_pressed: EnumSet<Key> = input.events.iter().filter_map(has_just_pressed).collect();

        Self {
            pressed,
            just_pressed,
        }
    }

    pub fn is_pressed(&self, key: Key) -> bool {
        self.pressed.contains(key)
    }

    pub fn just_pressed(&self, key: Key) -> bool {
        self.just_pressed.contains(key)
    }
}

fn has_just_pressed(event: &egui::Event) -> Option<Key> {
    match event {
        egui::Event::Key {
            key,
            pressed: true,
            repeat: false,
            ..
        } => Key::try_from(key).ok(),
        _ => None,
    }
}

impl TryFrom<&egui::Key> for Key {
    type Error = ();

    fn try_from(value: &egui::Key) -> Result<Self, Self::Error> {
        match value {
            egui::Key::ArrowUp => Ok(Key::ArrowUp),
            egui::Key::ArrowDown => Ok(Key::ArrowDown),
            egui::Key::ArrowLeft => Ok(Key::ArrowLeft),
            egui::Key::ArrowRight => Ok(Key::ArrowRight),
            egui::Key::Space => Ok(Key::Space),
            egui::Key::W => Ok(Key::W),
            egui::Key::A => Ok(Key::A),
            egui::Key::S => Ok(Key::S),
            egui::Key::D => Ok(Key::D),
            egui::Key::V => Ok(Key::V),
            _ => Err(()),
        }
    }
}
