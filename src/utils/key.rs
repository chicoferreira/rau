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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardState {
    pressed_keys: EnumSet<Key>,
}

impl KeyboardState {
    pub fn empty() -> Self {
        Self {
            pressed_keys: EnumSet::empty(),
        }
    }

    pub fn from_egui_input(input: &egui::InputState) -> Self {
        let mut pressed_keys: EnumSet<Key> = input
            .keys_down
            .iter()
            .filter_map(|k| Key::try_from(k).ok())
            .collect();

        if input.modifiers.shift {
            pressed_keys.insert(Key::Shift);
        }

        Self { pressed_keys }
    }

    pub fn is_pressed(&self, key: Key) -> bool {
        self.pressed_keys.contains(key)
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
            _ => Err(()),
        }
    }
}
