use crate::project::{
    TextureId,
    recreate::{Recreatable, RecreateResult, RecreateTracker},
    storage::Storage,
    texture::Texture,
};

#[derive(Clone, Copy)]
pub struct TextureViewProjectView<'a> {
    pub textures: &'a Storage<TextureId, Texture>,
}

pub struct TextureView {
    label: String,
    format: Option<TextureViewFormat>,
    dimension: Option<wgpu::TextureViewDimension>,
    array_layer_count: Option<u32>,
    texture_id: TextureId,
    inner: wgpu::TextureView,
    dirty: bool,
}

/// As currently the texture view format is only allowed to change by srgb-ness
/// This will allow the user to easily specify it
///
/// Check [`wgpu::wgt::TextureDescriptor::view_formats`]
#[derive(Debug, Clone, Copy)]
pub enum TextureViewFormat {
    Srgb,
    Linear,
}

impl TextureView {
    // Change this to descriptor to avoid huge constructors
    pub fn new(
        project: &TextureViewProjectView,
        label: String,
        texture_id: TextureId,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
        array_layer_count: Option<u32>,
    ) -> TextureView {
        let inner = Self::create_view(
            project,
            &label,
            texture_id,
            format,
            dimension,
            array_layer_count,
        );

        TextureView {
            label,
            format,
            dimension,
            array_layer_count,
            texture_id,
            inner,
            dirty: false,
        }
    }

    pub fn inner(&self) -> &wgpu::TextureView {
        &self.inner
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    fn create_view(
        project: &TextureViewProjectView,
        label: &str,
        texture_id: TextureId,
        format: Option<TextureViewFormat>,
        dimension: Option<wgpu::TextureViewDimension>,
        array_layer_count: Option<u32>,
    ) -> wgpu::TextureView {
        let texture = project
            .textures
            .get(texture_id)
            .expect("deal with this later");

        let wgpu_format = format.as_ref().map(|format| match format {
            TextureViewFormat::Srgb => texture.format().add_srgb_suffix(),
            TextureViewFormat::Linear => texture.format().remove_srgb_suffix(),
        });

        texture.inner().create_view(&wgpu::TextureViewDescriptor {
            label: Some(&label),
            format: wgpu_format,
            dimension,
            array_layer_count,
            ..Default::default()
        })
    }
}

impl Recreatable for TextureView {
    type Context<'a> = TextureViewProjectView<'a>;

    fn recreate<'a>(
        &mut self,
        context: &mut Self::Context<'a>,
        tracker: &RecreateTracker,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> RecreateResult {
        if !self.dirty && !tracker.was_recreated(self.texture_id) {
            return RecreateResult::Unchanged;
        }
        self.inner = Self::create_view(
            context,
            &self.label,
            self.texture_id,
            self.format,
            self.dimension,
            self.array_layer_count,
        );
        RecreateResult::Recreated
    }
}
