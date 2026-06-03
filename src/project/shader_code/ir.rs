#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScalarKind {
    F32,
    I32,
    U32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TexDim {
    D1,
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sampled {
    Float,
    Sint,
    Uint,
    Depth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
    Atomic,
}

#[derive(Clone, PartialEq)]
pub enum ShaderType {
    Scalar(ScalarKind),
    Vector {
        size: u8,
        scalar: ScalarKind,
    },
    Matrix {
        cols: u8,
        rows: u8,
        scalar: ScalarKind,
    },
    Texture {
        dim: TexDim,
        sampled: Sampled,
    },
    StorageTexture {
        dim: TexDim,
        format: Option<wgpu::TextureFormat>,
        access: Access,
    },
    Sampler {
        comparison: bool,
    },
    Struct(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Uniform,
    Texture,
    Sampler,
    StorageTexture,
}

pub struct ShaderField {
    pub name: String,
    pub ty: ShaderType,
    /// `@location(n)` for vertex inputs; `None` for plain struct fields.
    pub location: Option<u32>,
}

pub struct ShaderStruct {
    pub name: String,
    pub fields: Vec<ShaderField>,
}

pub struct ShaderBinding {
    /// The `@group` index, or `None` when not yet known (e.g. a bind group not
    /// yet assigned to a pipeline slot), rendered as a `_` placeholder.
    pub group: Option<u32>,
    pub binding: u32,
    pub kind: BindingKind,
    pub name: String,
    pub ty: ShaderType,
}

pub enum ShaderItem {
    Struct(ShaderStruct),
    Binding(ShaderBinding),
    Comment(String),
}

#[derive(Default)]
pub struct ShaderModule {
    items: Vec<ShaderItem>,
}

impl ShaderModule {
    pub fn add_struct(&mut self, definition: ShaderStruct) {
        self.items.push(ShaderItem::Struct(definition));
    }

    pub fn add_binding(&mut self, binding: ShaderBinding) {
        self.items.push(ShaderItem::Binding(binding));
    }

    pub fn comment(&mut self, text: impl Into<String>) {
        self.items.push(ShaderItem::Comment(text.into()));
    }

    pub fn items(&self) -> &[ShaderItem] {
        &self.items
    }
}
