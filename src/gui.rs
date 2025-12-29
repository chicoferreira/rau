pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        window: &winit::window::Window,
    ) -> Self {
        let egui_context = egui::Context::default();

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024),
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            egui_wgpu::RendererOptions::default(),
        );

        Self {
            state: egui_state,
            renderer: egui_renderer,
        }
    }

    pub fn render<F>(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        render_egui: F,
    ) where
        F: FnOnce(&egui::Context) -> (),
    {
        let raw_input = self.state.take_egui_input(window);
        let full_output = {
            let egui_context = self.state.egui_ctx();
            egui_context.begin_pass(raw_input);
            render_egui(egui_context);
            egui_context.set_pixels_per_point(screen_descriptor.pixels_per_point);
            egui_context.end_pass()
        };

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let egui_context = self.state.egui_ctx();

        let meshes = egui_context.tessellate(full_output.shapes, egui_context.pixels_per_point());
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &meshes, &screen_descriptor);

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            label: Some("egui render pass"),
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        self.renderer.render(
            &mut render_pass.forget_lifetime(),
            &meshes,
            &screen_descriptor,
        );

        for ele in &full_output.textures_delta.free {
            self.renderer.free_texture(ele);
        }
    }

    pub fn handle_input(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }
}
