use crate::project::Project;
use crate::renderer::Renderer;
use anyhow::Context;
use pollster::FutureExt;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

pub struct App {
    current_project: Project,
    renderer: Option<Renderer>,
}

impl App {
    pub fn new(project: Project) -> App {
        App {
            current_project: project,
            renderer: None,
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
        let window_attributes = winit::window::WindowAttributes::default().with_title("Rau");
        let window = event_loop.create_window(window_attributes).unwrap();
        let renderer = Renderer::new(window, &self.current_project).block_on();
        match renderer {
            Ok(renderer) => {
                self.renderer = Some(renderer);
            }
            Err(e) => {
                eprintln!("{:?}", e);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.window().request_redraw();

                    match renderer.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if it is lost or outdated
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            renderer.resize(renderer.window().inner_size())
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                        Err(wgpu::SurfaceError::Other) => log::error!("Other surface error"),
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size);
                }
            }
            _ => {}
        }
        if let Some(renderer) = &mut self.renderer {
            renderer.handle_input(&event);
        }
    }
}
