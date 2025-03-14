use crate::renderer::{gui, Renderer};

pub struct EguiRenderer {
    pub context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        output_depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
        window: &winit::window::Window,
    ) -> Self {
        let context = egui::Context::default();

        #[cfg(target_arch = "wasm32")]
        {
            context.set_pixels_per_point(window.scale_factor() as f32);
        }

        let id = context.viewport_id();
        let state = egui_winit::State::new(context.clone(), id, &window, None, None, None);

        let renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            false,
        );

        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn handle_input(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    pub fn draw(
        renderer: &mut Renderer,
        encoder: &mut wgpu::CommandEncoder,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    ) {
        let raw_input = renderer.egui.state.take_egui_input(&renderer.window);

        renderer.egui.context.begin_pass(raw_input);
        gui::render_gui(renderer);
        let full_output = renderer.egui.context.end_pass();

        renderer
            .egui
            .state
            .handle_platform_output(&renderer.window, full_output.platform_output);

        let tris = renderer
            .egui
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            renderer.egui.renderer.update_texture(
                &renderer.device,
                &renderer.queue,
                *id,
                image_delta,
            );
        }

        renderer.egui.renderer.update_buffers(
            &renderer.device,
            &renderer.queue,
            encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: window_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.egui.renderer.render(
                &mut render_pass.forget_lifetime(),
                &tris,
                &screen_descriptor,
            );
        }

        for x in &full_output.textures_delta.free {
            renderer.egui.renderer.free_texture(x);
        }
    }
}
