use pollster::FutureExt;

use crate::project::ProjectResourceId;

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("invalid resource: {0:?}")]
    InvalidResource(ProjectResourceId),
    #[error(transparent)]
    WgpuError(#[from] wgpu::Error),
    #[error("shader parse error: {0}")]
    ShaderParseError(#[from] naga::front::wgsl::ParseError),
    #[error("shader compilation error: {0}")]
    ShaderCompilationError(#[from] naga::WithSpan<naga::valid::ValidationError>),
    #[error("file load error: {0}")]
    FileLoadError(anyhow::Error),
    #[error(transparent)]
    ImageParseError(#[from] image::ImageError),
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
        self.inner
            .pop()
            .block_on()
            .map_or(Ok(()), |e| Err(e.into()))
    }
}
