use std::sync::mpsc::TryRecvError;
#[cfg(target_arch = "wasm32")]
use std::{future::Future, pin::Pin};

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

#[cfg(not(target_arch = "wasm32"))]
pub fn new_handler() -> (WgpuErrorScopeWaiter, WgpuErrorScopeReceiver) {
    let (main_thread_sender, receiver) = std::sync::mpsc::channel();

    let waiter = WgpuErrorScopeWaiter { main_thread_sender };
    let receiver = WgpuErrorScopeReceiver { receiver };

    (waiter, receiver)
}

#[cfg(target_arch = "wasm32")]
pub fn new_handler() -> (WgpuErrorScopeWaiter, WgpuErrorScopeReceiver) {
    let (main_thread_sender, receiver) = std::sync::mpsc::channel();
    let (wasm_sender, wasm_receiver) = tokio::sync::mpsc::unbounded_channel();

    spawn_wasm_error_poller_task(wasm_receiver, main_thread_sender.clone());

    let waiter = WgpuErrorScopeWaiter { wasm_sender };
    let receiver = WgpuErrorScopeReceiver { receiver };

    (waiter, receiver)
}

pub struct WgpuErrorScopeWaiter {
    #[cfg(not(target_arch = "wasm32"))]
    main_thread_sender: std::sync::mpsc::Sender<ErrorScopeResult>,
    #[cfg(target_arch = "wasm32")]
    wasm_sender: tokio::sync::mpsc::UnboundedSender<(ResourceId, Revision, WgpuErrorFuture)>,
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
        let future = Box::pin(scope.inner.pop());
        if let Err(err) = self.wasm_sender.send((resource_id, revision, future)) {
            log::error!(
                "Couldn't send popped scope future from {:?} (at {:?}) to the wasm error poller task: {:?}",
                resource_id,
                revision,
                err
            )
        }
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

#[cfg(target_arch = "wasm32")]
type WgpuErrorFuture = Pin<Box<dyn Future<Output = Option<wgpu::Error>> + 'static>>;

#[cfg(target_arch = "wasm32")]
fn spawn_wasm_error_poller_task(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<(ResourceId, Revision, WgpuErrorFuture)>,
    main_thread_sender: std::sync::mpsc::Sender<ErrorScopeResult>,
) {
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            while let Some((resource_id, revision, future)) = receiver.recv().await {
                let error = future.await;
                send_result(&main_thread_sender, resource_id, revision, error);
            }
        }
    });
}
