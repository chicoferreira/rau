use crate::{
    project::{DimensionId, ProjectResource},
    ui::Size2d,
};

pub struct Dimension {
    pub label: String,
    pub size: Size2d,
}

impl ProjectResource for Dimension {
    type Id = DimensionId;

    fn label(&self) -> &str {
        &self.label
    }
}
