use crate::{
    error::{AppResult, WgpuErrorScope},
    project::{
        ProjectResource, TextureId, TextureViewId,
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
    texture_id: Option<TextureId>,
    inner: Option<wgpu::TextureView>,
    egui_id: Option<egui::TextureId>,
    dirty: bool,
    has_error: bool,
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
        ctx: &mut TextureViewCreationContext,
        label: String,
        texture_id: Option<TextureId>,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> AppResult<TextureView> {
        let (inner, egui_id) = match texture_id {
            Some(texture_id) => {
                let texture = ctx.textures.get(texture_id)?;
                let inner = Self::create_view(&label, ctx.device, texture, format, dimension)?;

                let egui_id = (ALLOWED_EGUI_FORMATS.contains(&texture.format())).then(|| {
                    ctx.egui_renderer.register_egui_texture(
                        ctx.device,
                        &inner,
                        wgpu::FilterMode::Linear,
                    )
                });

                (Some(inner), egui_id)
            }
            None => (None, None),
        };

        Ok(TextureView {
            label,
            format,
            dimension,
            texture_id,
            inner,
            egui_id,
            dirty: false,
            has_error: false,
        })
    }

    pub fn inner(&self) -> &Option<wgpu::TextureView> {
        &self.inner
    }

    pub fn texture_id(&self) -> Option<TextureId> {
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

    pub fn set_texture_id(&mut self, texture_id: Option<TextureId>) {
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
        device: &wgpu::Device,
        texture: &Texture,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> AppResult<wgpu::TextureView> {
        let scope = WgpuErrorScope::push(device);
        let wgpu_format = format.as_ref().map(|format| match format {
            TextureViewFormat::Srgb => texture.format().add_srgb_suffix(),
            TextureViewFormat::Linear => texture.format().remove_srgb_suffix(),
        });

        let view = texture.inner().create_view(&wgpu::TextureViewDescriptor {
            label: Some(&label),
            format: wgpu_format,
            dimension,
            ..Default::default()
        });
        scope.pop()?;

        Ok(view)
    }
}

impl ProjectResource for TextureView {
    fn label(&self) -> &str {
        &self.label
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
    ) -> AppResult<Option<ProjectEvent>> {
        if !self.dirty
            && !self.has_error
            && !self.texture_id.is_some_and(|texture_id| {
                tracker.happened(ProjectEvent::TextureRecreated(texture_id))
            })
        {
            return Ok(None);
        }

        let scope = WgpuErrorScope::push(context.device);
        match self.texture_id {
            Some(texture_id) => {
                let texture = context.textures.get(texture_id)?;
                let inner = Self::create_view(
                    &self.label,
                    context.device,
                    texture,
                    self.format,
                    self.dimension,
                )
                .inspect_err(|_| self.has_error = true)?;

                let has_correct_format = ALLOWED_EGUI_FORMATS.contains(&texture.format());

                self.egui_id = match (self.egui_id, has_correct_format) {
                    (Some(egui_id), true) => {
                        context.egui_renderer.update_egui_texture(
                            context.device,
                            &inner,
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
                        &inner,
                        wgpu::FilterMode::Linear,
                    )),
                    (None, false) => None,
                };
                self.inner = Some(inner);
            }
            None => {
                if let Some(egui_id) = self.egui_id.take() {
                    context.egui_renderer.remove_egui_texture(egui_id);
                }
                self.inner = None;
            }
        }
        scope.pop().inspect_err(|_| self.has_error = true)?;

        self.has_error = false;
        self.dirty = false;

        Ok(Some(ProjectEvent::TextureViewRecreated(id)))
    }
}
