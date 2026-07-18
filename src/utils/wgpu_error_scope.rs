use crate::{error::AppResult, utils::async_job::AsyncJob};

/// A pushed wgpu validation error scope, popped as an [`AsyncJob`] polled once
/// per frame rather than blocked on.
///
/// Native resolves at the first poll. On the web the browser settles the scope,
/// and how long it may take is unspecified; observed behaviour is that it is
/// settled by the next frame.
pub struct WgpuErrorScope {
    inner: wgpu::ErrorScopeGuard,
}

impl WgpuErrorScope {
    pub fn push(device: &wgpu::Device) -> Self {
        Self {
            inner: device.push_error_scope(wgpu::ErrorFilter::Validation),
        }
    }

    pub fn pop(self) -> AsyncJob<AppResult<()>> {
        let future = self.inner.pop();
        AsyncJob::new(async move {
            match future.await {
                Some(error) => Err(error.into()),
                None => Ok(()),
            }
        })
    }
}
