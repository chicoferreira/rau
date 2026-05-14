use std::{future::Future, sync::Arc};

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop};

use crate::error::AppResult;

pub struct WinitRunner<A: WindowApp + 'static> {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<A>>,
    app: Option<A>,
    pending_window_events: Vec<WindowEvent>,
}

impl<A: WindowApp + 'static> WinitRunner<A> {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &winit::event_loop::EventLoop<A>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            app: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            pending_window_events: Vec::new(),
        }
    }
}

impl<A: WindowApp + 'static> ApplicationHandler<A> for WinitRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = winit::window::Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::UnwrapThrowExt;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.app = Some(pollster::block_on(A::new(window)).expect("Failed creating app"));
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    let app = A::new(window).await.expect("Failed creating app");
                    assert!(proxy.send_event(app).is_ok())
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, event_loop: &ActiveEventLoop, mut app: A) {
        let pending = self.pending_window_events.drain(..);
        for event in pending {
            app.handle_window_event(event_loop, event);
        }

        self.app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let app = match &mut self.app {
            Some(app) => app,
            None => {
                self.pending_window_events.push(event);
                return;
            }
        };

        app.handle_window_event(event_loop, event);
    }
}

pub trait WindowApp: Sized {
    fn new(window: Arc<winit::window::Window>) -> impl Future<Output = AppResult<Self>>;

    fn handle_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent);
}
