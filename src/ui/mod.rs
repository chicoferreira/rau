mod bind_group_inspector;
pub mod components;
pub mod pane;
pub mod renderer;
mod uniform_inspector;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size2d {
    width: u32,
    height: u32,
}

impl Size2d {
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);

        Size2d { width, height }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
