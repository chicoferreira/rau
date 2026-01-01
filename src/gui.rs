pub struct EguiRenderer {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

pub struct EguiFrame {
    pub meshes: Vec<egui::ClippedPrimitive>,
    pub textures_delta: egui::TexturesDelta,
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

    pub fn handle<F>(
        &mut self,
        window: &winit::window::Window,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        run_ui: F,
    ) -> EguiFrame
    where
        F: FnOnce(&egui::Context) -> (),
    {
        let raw_input = self.state.take_egui_input(window);
        let full_output = {
            let egui_context = self.state.egui_ctx();
            egui_context.begin_pass(raw_input);
            run_ui(egui_context);
            egui_context.set_pixels_per_point(screen_descriptor.pixels_per_point);
            egui_context.end_pass()
        };

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let egui_context = self.state.egui_ctx();

        let meshes = egui_context.tessellate(full_output.shapes, egui_context.pixels_per_point());

        EguiFrame {
            meshes,
            textures_delta: full_output.textures_delta,
        }
    }

    pub fn render_egui_frame(
        &mut self,
        frame: &EguiFrame,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        clear_color: wgpu::Color,
    ) {
        for (id, image_delta) in &frame.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &frame.meshes, screen_descriptor);

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
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
            &frame.meshes,
            screen_descriptor,
        );

        for ele in &frame.textures_delta.free {
            self.renderer.free_texture(ele);
        }
    }

    pub fn register_egui_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
    ) -> egui::TextureId {
        self.renderer
            .register_native_texture(device, texture, texture_filter)
    }

    pub fn update_egui_texture(
        &mut self,
        device: &wgpu::Device,
        texture: &wgpu::TextureView,
        texture_filter: wgpu::FilterMode,
        id: egui::TextureId,
    ) {
        self.renderer
            .update_egui_texture_from_wgpu_texture(device, texture, texture_filter, id);
    }

    pub fn handle_input(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }
}
