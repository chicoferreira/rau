use crate::project::Project;
use crate::renderer::Renderer;
use anyhow::Context;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::ActiveEventLoop;

pub struct App {
    current_project: Project,
    renderer: Option<Renderer>,
    #[cfg(target_arch = "wasm32")]
    renderer_receiver: Option<futures::channel::oneshot::Receiver<Renderer>>,
}

impl App {
    pub fn new(project: Project) -> App {
        App {
            current_project: project,
            renderer: None,
            #[cfg(target_arch = "wasm32")]
            renderer_receiver: None,
        }
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let event_loop =
            winit::event_loop::EventLoop::new().context("Failed to create event loop")?;

        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        event_loop.run_app(self).context("Failed to run app")
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut window_attributes = winit::window::WindowAttributes::default();

        #[allow(unused_assignments)]
        #[cfg(not(target_arch = "wasm32"))]
        {
            window_attributes = window_attributes.with_title("Rau");
        }

        #[allow(unused_assignments)]
        let mut canvas_size = (0, 0);

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            canvas_size = (canvas.client_width(), canvas.client_height());
            window_attributes = window_attributes.with_canvas(Some(canvas));
        }

        let window = event_loop.create_window(window_attributes).unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        {
            canvas_size = (window.inner_size().width, window.inner_size().height);
            self.renderer = Some(
                pollster::block_on(Renderer::new(
                    window,
                    &self.current_project,
                    canvas_size.into(),
                ))
                .expect("Failed to initialize renderer"),
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let (sender, receiver) = futures::channel::oneshot::channel();
            self.renderer_receiver = Some(receiver);

            let project = self.current_project.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let renderer = Renderer::new(window, &project, canvas_size.into())
                    .await
                    .expect("Failed to initialize renderer");

                if sender.send(renderer).is_err() {
                    log::error!("Failed to send renderer to main thread");
                }
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            let mut renderer_received = false;
            if let Some(receiver) = self.renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    renderer.window().request_redraw();
                    self.renderer = Some(renderer);
                    renderer_received = true;
                }
            }
            if renderer_received {
                self.renderer_receiver = None;
            }
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        renderer.handle_window_event(&event, event_loop);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        renderer.handle_device_event(&event);
    }
}
