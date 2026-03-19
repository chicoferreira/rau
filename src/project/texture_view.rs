use crate::{
    project::{
        TextureId, TextureViewId,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
        storage::Storage,
        texture::Texture,
    },
    ui::renderer::EguiRenderer,
};

pub struct TextureViewCreationContext<'a> {
    pub textures: &'a Storage<TextureId, Texture>,
    pub egui_renderer: &'a mut EguiRenderer,
    pub device: &'a wgpu::Device,
}

pub struct TextureView {
    label: String,
    format: Option<TextureViewFormat>,
    dimension: Option<wgpu::TextureViewDimension>,
    texture_id: TextureId,
    inner: wgpu::TextureView,
    egui_id: Option<egui::TextureId>,
    dirty: bool,
}

/// As currently the texture view format is only allowed to change by srgb-ness
/// This will allow the user to easily specify it
///
/// Check [`wgpu::wgt::TextureDescriptor::view_formats`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureViewFormat {
    Srgb,
    Linear,
}

const ALLOWED_EGUI_FORMATS: &[wgpu::TextureFormat] = &[
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
];

impl TextureView {
    // Change this to descriptor to avoid huge constructors
    pub fn new(
        ctx: TextureViewCreationContext,
        label: String,
        texture_id: TextureId,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> TextureView {
        let texture = ctx.textures.get(texture_id).expect("deal with this later");
        let inner = Self::create_view(&label, texture, format, dimension);

        let egui_id = (ALLOWED_EGUI_FORMATS.contains(&texture.format())).then(|| {
            ctx.egui_renderer
                .register_egui_texture(ctx.device, &inner, wgpu::FilterMode::Linear)
        });

        TextureView {
            label,
            format,
            dimension,
            texture_id,
            inner,
            egui_id,
            dirty: false,
        }
    }

    pub fn inner(&self) -> &wgpu::TextureView {
        &self.inner
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id
    }

    pub fn format(&self) -> Option<TextureViewFormat> {
        self.format
    }

    pub fn dimension(&self) -> Option<wgpu::TextureViewDimension> {
        self.dimension
    }

    /// Returns the egui texture ID.
    /// Only returns `Some` if the texture format is `Rgba8UnormSrgb` due to egui texture format requirements.
    pub fn egui_id(&self) -> Option<egui::TextureId> {
        self.egui_id
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.dirty = true;
    }

    pub fn set_texture_id(&mut self, texture_id: TextureId) {
        self.texture_id = texture_id;
        self.dirty = true;
    }

    pub fn set_format(&mut self, format: Option<TextureViewFormat>) {
        self.format = format;
        self.dirty = true;
    }

    pub fn set_dimension(&mut self, dimension: Option<wgpu::TextureViewDimension>) {
        self.dimension = dimension;
        self.dirty = true;
    }

    fn create_view(
        label: &str,
        texture: &Texture,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> wgpu::TextureView {
        let wgpu_format = format.as_ref().map(|format| match format {
            TextureViewFormat::Srgb => texture.format().add_srgb_suffix(),
            TextureViewFormat::Linear => texture.format().remove_srgb_suffix(),
        });

        texture.inner().create_view(&wgpu::TextureViewDescriptor {
            label: Some(&label),
            format: wgpu_format,
            dimension,
            ..Default::default()
        })
    }
}

impl Recreatable for TextureView {
    type Context<'a> = TextureViewCreationContext<'a>;
    type Id = TextureViewId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        context: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
    ) -> Option<ProjectEvent> {
        if !self.dirty && !tracker.happened(ProjectEvent::TextureRecreated(self.texture_id)) {
            return None;
        }

        let texture = context
            .textures
            .get(self.texture_id)
            .expect("deal with this later");

        self.inner = Self::create_view(&self.label, texture, self.format, self.dimension);

        let has_correct_format = ALLOWED_EGUI_FORMATS.contains(&texture.format());

        self.egui_id = match (self.egui_id, has_correct_format) {
            (Some(egui_id), true) => {
                context.egui_renderer.update_egui_texture(
                    context.device,
                    &self.inner,
                    wgpu::FilterMode::Linear,
                    egui_id,
                );
                Some(egui_id)
            }
            (Some(egui_id), false) => {
                context.egui_renderer.remove_egui_texture(egui_id);
                None
            }
            (None, true) => Some(context.egui_renderer.register_egui_texture(
                context.device,
                &self.inner,
                wgpu::FilterMode::Linear,
            )),
            (None, false) => None,
        };

        self.dirty = false;

        Some(ProjectEvent::TextureViewRecreated(id))
    }
}
