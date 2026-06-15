#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

use crate::project::{ResourceId, paths::FilePath};

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The resource with the given ID is invalid.
    #[error("Invalid resource {0:?}")]
    InvalidResource(ResourceId),
    /// The resource has uninitialized fields.
    #[error("Resource has uninitialized field “{0}”.")]
    UninitializedFields(String),
    /// The pipeline has more bind group layouts than wgpu supports.
    #[error("Pipeline uses {count} bind group layouts, but this device supports at most {max}.")]
    BindGroupLayoutLimitExceeded { count: usize, max: usize },
    #[error(
        "Model “{model_label}” material {material_index} has a bind group layout that doesn't match material {expected_material_index}."
    )]
    ModelMaterialBindGroupLayoutMismatch {
        model_label: String,
        expected_material_index: usize,
        material_index: usize,
    },
    /// The current renderer does not support a feature required by the resource.
    #[error("The “{0}” feature isn't supported by the current renderer.")]
    UnsupportedRendererFeature(&'static str),
    /// Access to a resource that is erroring
    #[error("Resource {0:?} is in an error state.")]
    WaitingForErroredResource(ResourceId),
    /// A WGPU error occurred.
    #[error(transparent)]
    WgpuError(#[from] wgpu::Error),
    #[error(transparent)]
    WgpuRequestAdapterError(#[from] wgpu::RequestAdapterError),
    #[error(transparent)]
    WgpuRequestDeviceError(#[from] wgpu::RequestDeviceError),
    #[error("Unsupported shader extension {0:?}. Expected .wgsl, .vert, .frag or .comp.")]
    UnsupportedShaderExtension(String),
    /// A shader parse error occurred. Holds a message formatted with source
    /// location (line numbers and offending source snippet).
    #[error("Shader parse error:\n{0}")]
    ShaderParseError(String),
    /// A shader validation error occurred. Holds a message formatted with source
    /// location (line numbers and offending source snippet).
    #[error("Shader validation error:\n{0}")]
    ShaderCompilationError(String),
    /// A file load error occurred.
    #[error("Failed to load file: {0}")]
    FileLoadError(#[from] std::io::Error),
    /// The file was not found.
    #[error("File not found: {0}")]
    FileNotFound(FilePath),
    /// The file or directory already exists.
    #[error("Path already exists: {0}")]
    PathAlreadyExists(FilePath),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("This folder already contains a project.json: {0}")]
    ProjectJsonAlreadyExists(PathBuf),
    #[cfg(target_arch = "wasm32")]
    #[error("A project named “{0}” already exists.")]
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
    #[error("File contains invalid UTF-8: {0}")]
    FileNotValidUtf8(FilePath),
    #[cfg(target_arch = "wasm32")]
    #[error("Browser error: {0}")]
    BrowserError(String),
    #[error("Failed to fetch: {0}")]
    FetchError(ehttp::Error),
    #[error("Failed to parse URL: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Invalid path segment: {0:?}")]
    InvalidPathSegment(String),
    #[error("Invalid create project form: {0}")]
    InvalidCreateProjectForm(&'static str),
    #[cfg(target_arch = "wasm32")]
    #[error("Invalid URL parameters: {0}")]
    InvalidUrlParameters(String),
    #[error("Failed to capture viewport: {0}")]
    CaptureError(String),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Config directory is unavailable.")]
    ConfigDirectoryUnavailable,
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Invalid project path: {0:?}")]
    InvalidProjectPath(PathBuf),
    #[error("Missing project.json.")]
    MissingProjectJson,
    #[error("Failed to serialize/deserialize JSON: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Failed to deserialize TOML: {0}")]
    TomlDeserializeError(#[from] toml::de::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Failed to serialize TOML: {0}")]
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
