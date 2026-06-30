use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size2d {
    width: u32,
    height: u32,
}

impl Default for Size2d {
    fn default() -> Self {
        Self::new(1, 1)
    }
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

impl From<wgpu::Extent3d> for Size2d {
    fn from(value: wgpu::Extent3d) -> Self {
        Self {
            width: value.width,
            height: value.height,
        }
    }
}

impl Serialize for Size2d {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        [self.width, self.height].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Size2d {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let [width, height] = <[u32; 2]>::deserialize(deserializer)?;
        Ok(Self::new(width, height))
    }
}
