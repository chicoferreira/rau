use crate::{
    error::{AppError, AppResult},
    project::{
        Creatable, ProjectResource, TextureId, TextureViewId,
        resource::texture::{Texture, TextureRuntime},
        storage::{RuntimeStorage, Storage},
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    ui::renderer::EguiRenderer,
};

pub struct TextureViewCreationContext<'a> {
    pub textures: &'a Storage<Texture>,
    pub textures_runtime: &'a RuntimeStorage<Texture>,
    pub egui_renderer: &'a mut EguiRenderer,
    pub device: &'a wgpu::Device,
}

pub struct TextureView {
    label: String,
    format: Option<TextureViewFormat>,
    dimension: Option<wgpu::TextureViewDimension>,
    texture_id: Option<TextureId>,
    revision: Revision,
}

pub struct TextureViewRuntime {
    inner: wgpu::TextureView,
    egui_id: Option<egui::TextureId>,
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

// TODO: change this to a better filter, i think it should work with every RGBA texture format
const ALLOWED_EGUI_FORMATS: &[wgpu::TextureFormat] = &[
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgba16Float,
];

impl TextureView {
    pub fn new(
        label: impl Into<String>,
        texture_id: Option<TextureId>,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> TextureView {
        TextureView {
            label: label.into(),
            format,
            dimension,
            texture_id,
            revision: Revision::default(),
        }
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

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.revision.increase();
    }

    pub fn set_texture_id(&mut self, texture_id: Option<TextureId>) {
        self.texture_id = texture_id;
        self.revision.increase();
    }

    pub fn set_format(&mut self, format: Option<TextureViewFormat>) {
        self.format = format;
        self.revision.increase();
    }

    pub fn set_dimension(&mut self, dimension: Option<wgpu::TextureViewDimension>) {
        self.dimension = dimension;
        self.revision.increase();
    }

    fn create_view(
        label: &str,
        texture: &Texture,
        runtime: &TextureRuntime,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
    ) -> wgpu::TextureView {
        let wgpu_format = format.as_ref().map(|format| match format {
            TextureViewFormat::Srgb => texture.format().add_srgb_suffix(),
            TextureViewFormat::Linear => texture.format().remove_srgb_suffix(),
        });

        let view = runtime.inner().create_view(&wgpu::TextureViewDescriptor {
            label: Some(&label),
            format: wgpu_format,
            dimension,
            ..Default::default()
        });

        view
    }
}

impl TextureViewRuntime {
    pub fn inner(&self) -> &wgpu::TextureView {
        &self.inner
    }

    /// Returns the egui texture ID.
    /// Only returns `Some` if the texture format is `Rgba8UnormSrgb` due to egui texture format requirements.
    pub fn egui_id(&self) -> Option<egui::TextureId> {
        self.egui_id
    }
}

impl Creatable for TextureView {
    const DEFAULT_LABEL: &'static str = "Texture View";

    fn create(label: String) -> Self {
        Self::new(label, None, None, None)
    }
}

impl ProjectResource for TextureView {
    type Id = TextureViewId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for TextureView {
    type Context<'a> = TextureViewCreationContext<'a>;
    type Runtime = TextureViewRuntime;

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let previous_egui_id = previous.as_ref().and_then(|runtime| runtime.egui_id);

        let texture_id = self.texture_id.ok_or(AppError::UninitializedFields)?;

        let texture = ctx.textures.get(texture_id)?;
        let runtime_texture = ctx.textures_runtime.get_init(texture_id)?;

        let inner = Self::create_view(
            &self.label,
            texture,
            runtime_texture,
            self.format,
            self.dimension,
        );

        let has_correct_format = ALLOWED_EGUI_FORMATS.contains(&texture.format());

        let egui_id = match (previous_egui_id, has_correct_format) {
            (Some(egui_id), true) => {
                ctx.egui_renderer.update_egui_texture(
                    ctx.device,
                    &inner,
                    wgpu::FilterMode::Linear,
                    egui_id,
                );
                Some(egui_id)
            }
            (Some(egui_id), false) => {
                ctx.egui_renderer.remove_egui_texture(egui_id);
                None
            }
            (None, true) => Some(ctx.egui_renderer.register_egui_texture(
                ctx.device,
                &inner,
                wgpu::FilterMode::Linear,
            )),
            (None, false) => None,
        };

        Ok(SyncOutcome::Changed(TextureViewRuntime { inner, egui_id }))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        let Some(texture_id) = self.texture_id else {
            return false;
        };
        tracker.was_changed(texture_id)
    }
}
