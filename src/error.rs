use crate::project::{ResourceId, file::ProjectFilePath};

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The resource with the given ID is invalid.
    #[error("invalid resource: {0:?}")]
    InvalidResource(ResourceId),
    /// The resource has uninitialized fields.
    #[error("resource has uninitialized fields")]
    UninitializedFields,
    /// The render pipeline has more bind group layouts than wgpu supports.
    #[error("bind group layout count {count} exceeds render pass bind group limit {max}")]
    BindGroupLayoutLimitExceeded { count: usize, max: usize },
    /// The resource where this error occurred is not yet initialized.
    #[error("resource is not yet initialized: {0:?}")]
    WaitingForUninitResource(ResourceId),
    /// Access to a resource that is erroring
    #[error("resource is erroring: {0:?}")]
    WaitingForErroredResource(ResourceId),
    /// The resource is waiting for a pending sync operation.
    #[error("resource is waiting for pending sync: {0:?}")]
    WaitingForPendingResource(ResourceId),
    /// A WGPU error occurred.
    #[error(transparent)]
    WgpuError(#[from] wgpu::Error),
    #[error(transparent)]
    WgpuRequestAdapterError(#[from] wgpu::RequestAdapterError),
    #[error(transparent)]
    WgpuRequestDeviceError(#[from] wgpu::RequestDeviceError),
    /// A shader parse error occurred.
    #[error("shader parse error: {0}")]
    ShaderParseError(#[from] naga::front::wgsl::ParseError),
    /// A shader compilation error occurred.
    #[error("shader compilation error: {0}")]
    ShaderCompilationError(#[from] naga::WithSpan<naga::valid::ValidationError>),
    /// A file load error occurred.
    #[error("file load error: {0}")]
    FileLoadError(#[from] std::io::Error),
    /// The file was not found.
    #[error("file not found: {0:?}")]
    FileNotFound(ProjectFilePath),
    /// An image parse error occurred.
    #[error(transparent)]
    ImageParseError(#[from] image::ImageError),
    /// An OBJ load error occurred.
    #[error(transparent)]
    ObjLoadError(#[from] tobj::LoadError),
    #[cfg(not(target_arch = "wasm32"))]
    #[error(transparent)]
    NotifyError(#[from] notify::Error),
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    IndexedDbOpenDbError(#[from] indexed_db_futures::error::OpenDbError),
    #[cfg(target_arch = "wasm32")]
    #[error(transparent)]
    IndexedDbError(#[from] indexed_db_futures::error::Error),
    #[cfg(target_arch = "wasm32")]
    #[error("file not valid utf8: {0:?}")]
    FileNotValidUtf8(ProjectFilePath),
    #[error("fetch error: {0}")]
    FetchError(#[from] reqwest::Error),
    #[error("url parse error: {0}")]
    UrlParseError(#[from] url::ParseError),
}
