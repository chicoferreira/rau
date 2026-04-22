use std::sync::mpsc::TryRecvError;

use crate::project::{ResourceId, sync::Revision};

pub struct WgpuErrorScope {
    inner: wgpu::ErrorScopeGuard,
}

impl WgpuErrorScope {
    pub fn push(device: &wgpu::Device) -> Self {
        Self {
            inner: device.push_error_scope(wgpu::ErrorFilter::Validation),
        }
    }
}

#[derive(Debug)]
pub struct ErrorScopeResult {
    pub resource_id: ResourceId,
    pub revision: Revision,
    pub error: Option<wgpu::Error>,
}

impl ErrorScopeResult {
    pub fn new(resource_id: ResourceId, revision: Revision, error: Option<wgpu::Error>) -> Self {
        Self {
            resource_id,
            revision,
            error,
        }
    }
}

pub fn new_handler() -> (WgpuErrorScopeWaiter, WgpuErrorScopeReceiver) {
    let (main_thread_sender, receiver) = std::sync::mpsc::channel();

    let waiter = WgpuErrorScopeWaiter { main_thread_sender };
    let receiver = WgpuErrorScopeReceiver { receiver };

    (waiter, receiver)
}

pub struct WgpuErrorScopeWaiter {
    main_thread_sender: std::sync::mpsc::Sender<ErrorScopeResult>,
}

pub struct WgpuErrorScopeReceiver {
    receiver: std::sync::mpsc::Receiver<ErrorScopeResult>,
}

impl WgpuErrorScopeWaiter {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn pop_error(&self, resource_id: ResourceId, revision: Revision, scope: WgpuErrorScope) {
        // In native this future should resolve immediately.
        let error = pollster::block_on(scope.inner.pop());
        send_result(&self.main_thread_sender, resource_id, revision, error);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn pop_error(&self, resource_id: ResourceId, revision: Revision, scope: WgpuErrorScope) {
        // In wasm the pop itself must happen immediately to preserve the
        // device's LIFO error-scope stack. Only awaiting the result is deferred.
        let future = scope.inner.pop();
        let main_thread_sender = self.main_thread_sender.clone();

        wasm_bindgen_futures::spawn_local(async move {
            let error = future.await;
            send_result(&main_thread_sender, resource_id, revision, error);
        });
    }
}

impl WgpuErrorScopeReceiver {
    pub fn try_next(&self) -> Option<ErrorScopeResult> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("WgpuErrorScopeReceiver::try_next called after sender side dropped")
            }
        }
    }
}

fn send_result(
    main_thread_sender: &std::sync::mpsc::Sender<ErrorScopeResult>,
    resource_id: ResourceId,
    revision: Revision,
    error: Option<wgpu::Error>,
) {
    let result = ErrorScopeResult::new(resource_id, revision, error);
    if let Err(err) = main_thread_sender.send(result) {
        log::error!(
            "Failed to send validation result of {:?} (at {:?}) back to the main thread: {:?}",
            resource_id,
            revision,
            err,
        )
    }
}
