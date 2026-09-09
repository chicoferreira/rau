use crate::{error::AppResult, utils::async_job::AsyncJob};

/// Pushed wgpu validation and internal error scopes, popped as an [`AsyncJob`] polled once
/// per frame rather than blocked on.
///
/// Native resolves at the first poll. On the web the browser settles the scope,
/// and how long it may take is unspecified; observed behaviour is that it is
/// settled by the next frame.
pub struct WgpuErrorScope {
    internal: wgpu::ErrorScopeGuard,
    validation: wgpu::ErrorScopeGuard,
}

impl WgpuErrorScope {
    pub fn push(device: &wgpu::Device) -> Self {
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
        Self {
            internal,
            validation,
        }
    }

    pub fn pop(self) -> AsyncJob<AppResult<()>> {
        let internal = self.internal.pop();
        let validation = self.validation.pop();
        AsyncJob::new(async move {
            let internal = internal.await;
            let validation = validation.await;
            match internal.or(validation) {
                Some(error) => Err(error.into()),
                None => Ok(()),
            }
        })
    }
}
