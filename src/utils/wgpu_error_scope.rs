use crate::{error::AppResult, utils::async_job::AsyncJob};

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
