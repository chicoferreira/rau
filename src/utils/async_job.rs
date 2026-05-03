use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

pub struct AsyncJob<T> {
    inner: Pin<Box<dyn Future<Output = T> + 'static>>,
}

impl<T> AsyncJob<T> {
    pub fn new(future: impl Future<Output = T> + 'static) -> Self {
        Self {
            inner: Box::pin(future),
        }
    }

    pub fn try_resolve(&mut self) -> Poll<T> {
        let mut cx = Context::from_waker(Waker::noop());
        self.inner.as_mut().poll(&mut cx)
    }
}

impl<T> Future for AsyncJob<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}

impl<T> std::fmt::Debug for AsyncJob<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncJob").finish_non_exhaustive()
    }
}
