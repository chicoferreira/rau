use std::future::Future;

use crate::utils::async_job::AsyncJob;

pub struct TaskSender<T> {
    sender: tokio::sync::oneshot::Sender<T>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn channel<T: Send + 'static>() -> (TaskSender<T>, AsyncJob<T>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let task = AsyncJob::new(async move {
        receiver
            .await
            .expect("background task must complete unless the runner panics")
    });

    (TaskSender { sender }, task)
}

#[cfg(target_arch = "wasm32")]
pub fn channel<T: 'static>() -> (TaskSender<T>, AsyncJob<T>) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let task = AsyncJob::new(async move {
        receiver
            .await
            .expect("background task must complete unless the runner panics")
    });

    (TaskSender { sender }, task)
}

impl<T> TaskSender<T> {
    pub fn send(self, result: T) {
        let _ = self.sender.send(result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_future<T: Send + 'static>(
    name: impl Into<String>,
    future: impl Future<Output = T> + Send + 'static,
) -> AsyncJob<T> {
    let (sender, task) = channel();

    std::thread::Builder::new()
        .name(name.into())
        .spawn(move || sender.send(pollster::block_on(future)))
        .expect("couldn't spawn background task thread");

    task
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_future<T: 'static>(
    _name: impl Into<String>,
    future: impl Future<Output = T> + 'static,
) -> AsyncJob<T> {
    let (sender, task) = channel();

    wasm_bindgen_futures::spawn_local(async move {
        sender.send(future.await);
    });

    task
}
