#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::project::{ResourceId, paths::FilePath};

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The resource with the given ID is invalid.
    #[error("invalid resource: {0:?}")]
    InvalidResource(ResourceId),
    /// The resource has uninitialized fields.
    #[error("Resource has uninitialized fields: {0}")]
    UninitializedFields(String),
    /// The pipeline has more bind group layouts than wgpu supports.
    #[error("bind group layout count {count} exceeds device bind group limit {max}")]
    BindGroupLayoutLimitExceeded { count: usize, max: usize },
    #[error(
        "model {model_label:?} material {material_index} bind group layout does not match material {expected_material_index} bind group layout"
    )]
    ModelMaterialBindGroupLayoutMismatch {
        model_label: String,
        expected_material_index: usize,
        material_index: usize,
    },
    /// The current renderer does not support a feature required by the resource.
    #[error("{0} feature is not supported by the current renderer")]
    UnsupportedRendererFeature(&'static str),
    /// Access to a resource that is erroring
    #[error("resource is erroring: {0:?}")]
    WaitingForErroredResource(ResourceId),
    /// A WGPU error occurred.
    #[error(transparent)]
    WgpuError(#[from] wgpu::Error),
    #[error(transparent)]
    WgpuRequestAdapterError(#[from] wgpu::RequestAdapterError),
    #[error(transparent)]
    WgpuRequestDeviceError(#[from] wgpu::RequestDeviceError),
    /// A shader parse error occurred. Holds a message formatted with source
    /// location (line numbers and offending source snippet).
    #[error("shader parse error:\n{0}")]
    ShaderParseError(String),
    /// A shader validation error occurred. Holds a message formatted with source
    /// location (line numbers and offending source snippet).
    #[error("shader validation error:\n{0}")]
    ShaderCompilationError(String),
    /// A file load error occurred.
    #[error("file load error: {0}")]
    FileLoadError(#[from] std::io::Error),
    /// The file was not found.
    #[error("file not found: {0:?}")]
    FileNotFound(FilePath),
    /// The file or directory already exists.
    #[error("path already exists: {0:?}")]
    PathAlreadyExists(FilePath),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("project folder already contains project.json: {0:?}")]
    ProjectJsonAlreadyExists(PathBuf),
    #[cfg(target_arch = "wasm32")]
    #[error("project name already exists: {0}")]
    ProjectNameAlreadyExists(String),
    /// An image parse error occurred.
    #[error(transparent)]
    ImageParseError(#[from] image::ImageError),
    /// An OBJ load error occurred.
    #[error(transparent)]
    ObjLoadError(#[from] tobj::LoadError),
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    NotifyError(#[from] notify::Error),
    #[error(transparent)]
    WinitEventLoopError(#[from] winit::error::EventLoopError),
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    IndexedDbOpenDbError(#[from] indexed_db_futures::error::OpenDbError),
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    IndexedDbError(#[from] indexed_db_futures::error::Error),
    #[cfg(target_arch = "wasm32")]
    #[error("file not valid utf8: {0:?}")]
    FileNotValidUtf8(FilePath),
    #[cfg(target_arch = "wasm32")]
    #[error("browser error: {0}")]
    BrowserError(String),
    #[error("fetch error: {0}")]
    FetchError(ehttp::Error),
    #[error("url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("invalid path segment: {0}")]
    InvalidPathSegment(String),
    #[error("invalid create project form: {0}")]
    InvalidCreateProjectForm(&'static str),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("config directory unavailable")]
    ConfigDirectoryUnavailable,
    #[cfg(not(target_arch = "wasm32"))]
    #[error("invalid project path: {0:?}")]
    InvalidProjectPath(PathBuf),
    #[error("missing project.json")]
    MissingProjectJson,
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("toml deserialize error: {0}")]
    TomlDeserializeError(#[from] toml::de::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("toml serialize error: {0}")]
    TomlSerializeError(#[from] toml::ser::Error),
}

impl AppError {
    pub fn uninit_field(field: impl Into<String>) -> Self {
        AppError::UninitializedFields(field.into())
    }
}

pub trait RequiredFieldExt {
    type Output;

    fn ok_or_uninit_field(self, field: impl Into<String>) -> AppResult<Self::Output>;
}

impl<T> RequiredFieldExt for Option<T> {
    type Output = T;

    fn ok_or_uninit_field(self, field: impl Into<String>) -> AppResult<Self::Output> {
        self.ok_or_else(|| AppError::uninit_field(field))
    }
}

impl From<std::convert::Infallible> for AppError {
    fn from(error: std::convert::Infallible) -> Self {
        match error {}
    }
}
