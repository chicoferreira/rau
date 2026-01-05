use crate::{gui, texture};

pub struct Viewport<C: ViewportContent> {
    texture: texture::Texture,
    texture_format: wgpu::TextureFormat,
    texture_id: egui::TextureId,
    width: u32,
    height: u32,
    content: C,
}

impl<C: ViewportContent> Viewport<C> {
    pub fn new(
        content: C,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        texture_format: wgpu::TextureFormat,
        egui_renderer: &mut gui::EguiRenderer,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = texture::Texture::create_texture(
            &device,
            Some("viewport_texture"),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            texture_format.remove_srgb_suffix(),
            &[texture_format.add_srgb_suffix()],
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            wgpu::TextureDimension::D2,
            wgpu::FilterMode::Linear,
        );
        let texture_id =
            egui_renderer.register_egui_texture(&device, &texture.view, wgpu::FilterMode::Linear);

        Viewport {
            texture,
            texture_format,
            texture_id,
            width,
            height,
            content,
        }
    }

    fn resize_target(
        &mut self,
        width: u32,
        height: u32,
        device: &wgpu::Device,
        egui_renderer: &mut gui::EguiRenderer,
    ) {
        let width = width.max(1);
        let height = height.max(1);
        self.width = width;
        self.height = height;

        self.texture = texture::Texture::create_texture(
            device,
            Some("viewport_texture"),
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            self.texture_format.remove_srgb_suffix(),
            &[self.texture_format.add_srgb_suffix()],
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            wgpu::TextureDimension::D2,
            wgpu::FilterMode::Linear,
        );
        egui_renderer.update_egui_texture(
            device,
            &self.texture.view,
            wgpu::FilterMode::Linear,
            self.texture_id,
        );
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Vec<ViewportEvent> {
        let mut events = Vec::new();
        let size_points = ui.available_size().max(egui::Vec2::new(1.0, 1.0));
        let pixels_per_point = ui.ctx().pixels_per_point();

        let requested_viewport_width = (size_points.x * pixels_per_point).round().max(1.0) as u32;
        let requested_viewport_height = (size_points.y * pixels_per_point).round().max(1.0) as u32;

        if requested_viewport_width != self.width || requested_viewport_height != self.height {
            events.push(ViewportEvent::Resize {
                width: requested_viewport_width,
                height: requested_viewport_height,
            });
        }

        let image = egui::Image::new(egui::load::SizedTexture::new(self.texture_id, size_points))
            .sense(egui::Sense::drag());

        let response = ui.add(image);

        if response.dragged_by(egui::PointerButton::Primary) {
            let delta_points = ui.input(|i| i.pointer.delta());
            if delta_points.x != 0.0 || delta_points.y != 0.0 {
                let delta_px = delta_points * pixels_per_point;
                events.push(ViewportEvent::Drag {
                    dx_px: delta_px.x,
                    dy_px: delta_px.y,
                });
            }
        }

        if response.hovered() {
            let scroll_points = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll_points != 0.0 {
                events.push(ViewportEvent::Scroll {
                    delta_y_px: scroll_points * pixels_per_point,
                });
            }
        }

        events
    }

    pub fn apply_events(
        &mut self,
        events: Vec<ViewportEvent>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        egui_renderer: &mut gui::EguiRenderer,
    ) {
        for event in events {
            match event {
                ViewportEvent::Resize { width, height } => {
                    self.resize_target(width, height, device, egui_renderer);
                }
                _ => {}
            }
            self.content.on_event(event, device, queue);
        }
    }

    pub fn update(&mut self, dt: instant::Duration, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.content
            .on_event(ViewportEvent::Frame { dt }, device, queue);
    }

    pub fn handle_keyboard(
        &mut self,
        key_code: winit::keyboard::KeyCode,
        element_state: winit::event::ElementState,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.content.on_event(
            ViewportEvent::Keyboard {
                key_code,
                element_state,
            },
            device,
            queue,
        );
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder) {
        self.content
            .render(encoder, &self.texture, self.texture_format);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ViewportEvent {
    Resize {
        width: u32,
        height: u32,
    },
    Scroll {
        delta_y_px: f32,
    },
    Drag {
        dx_px: f32,
        dy_px: f32,
    },
    Keyboard {
        key_code: winit::keyboard::KeyCode,
        element_state: winit::event::ElementState,
    },
    Frame {
        dt: instant::Duration,
    },
}

pub trait ViewportContent {
    fn on_event(&mut self, event: ViewportEvent, device: &wgpu::Device, queue: &wgpu::Queue);

    fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target_texture: &texture::Texture,
        target_texture_format: wgpu::TextureFormat,
    );
}
