use crate::project::ProjectResourceId;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The resource with the given ID is invalid.
    #[error("invalid resource: {0:?}")]
    InvalidResource(ProjectResourceId),
    /// The resource where this error occurred is not yet initialized.
    #[error("resource is not yet initialized")]
    UninitResource,
    /// A WGPU error occurred.
    #[error(transparent)]
    WgpuError(#[from] wgpu::Error),
    /// A shader parse error occurred.
    #[error("shader parse error: {0}")]
    ShaderParseError(#[from] naga::front::wgsl::ParseError),
    /// A shader compilation error occurred.
    #[error("shader compilation error: {0}")]
    ShaderCompilationError(#[from] naga::WithSpan<naga::valid::ValidationError>),
    /// A file load error occurred.
    #[error("file load error: {0}")]
    FileLoadError(anyhow::Error), // TODO: change this from anyhow to a more concrete file error
    /// An image parse error occurred.
    #[error(transparent)]
    ImageParseError(#[from] image::ImageError),
    /// An OBJ load error occurred.
    #[error(transparent)]
    ObjLoadError(#[from] tobj::LoadError),
}

pub struct SourcedError {
    pub source: Option<ProjectResourceId>,
    pub error: AppError,
}

impl SourcedError {
    pub fn new_unknown(error: AppError) -> Self {
        Self {
            source: None,
            error,
        }
    }

    pub fn new(source: ProjectResourceId, error: AppError) -> Self {
        Self {
            source: Some(source),
            error,
        }
    }
}

pub struct WgpuErrorScope {
    inner: wgpu::ErrorScopeGuard,
}

impl WgpuErrorScope {
    pub fn push(device: &wgpu::Device) -> Self {
        Self {
            inner: device.push_error_scope(wgpu::ErrorFilter::Validation),
        }
    }

    pub fn pop(self) -> AppResult<()> {
        let error = self.inner.pop();

        #[cfg(not(target_arch = "wasm32"))]
        {
            pollster::block_on(error).map_or(Ok(()), |e| Err(e.into()))
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_bindgen_futures::spawn_local(async move {
                if let Some(error) = error.await {
                    // TODO: send to main thread
                    log::error!("Unhandled WGPU error: {:?}", error);
                }
            });
            Ok(())
        }
    }
}
