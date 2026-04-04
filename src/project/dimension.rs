use crate::{project::ProjectResource, ui::Size2d};

pub struct Dimension {
    pub label: String,
    pub size: Size2d,
}

impl ProjectResource for Dimension {
    fn label(&self) -> &str {
        &self.label
    }
}
