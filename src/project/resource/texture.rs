use image::GenericImageView;

use crate::{
    error::{AppError, AppResult},
    project::{
        DimensionId, ProjectResource, TextureId,
        resource::dimension::Dimension,
        storage::Storage,
        sync::SyncTracker,
        sync::{Revision, SyncOutcome, SyncResource},
    },
};

#[derive(Clone, Copy)]
pub struct TextureCreationContext<'a> {
    pub dimensions: &'a Storage<Dimension>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct Texture {
    label: String,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    source: TextureSource,
    revision: Revision,
}

pub struct TextureRuntime {
    inner: wgpu::Texture,
}

#[derive(Clone, PartialEq)]
pub enum TextureSource {
    // Grab size from dimension
    Dimension(Option<DimensionId>),
    Image(image::DynamicImage), // TODO: change this to image_id once we have it in the project
    Manual { size: wgpu::Extent3d },
}

impl Texture {
    pub fn new(
        label: impl Into<String>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: TextureSource,
    ) -> Texture {
        Texture {
            label: label.into(),
            format,
            usage,
            source,
            revision: Revision::default(),
        }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub fn usage(&self) -> wgpu::TextureUsages {
        self.usage
    }

    pub fn source(&self) -> &TextureSource {
        &self.source
    }

    pub fn set_label(&mut self, label: String) {
        self.label = label;
        self.revision.increase();
    }

    pub fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = format;
        self.revision.increase();
    }

    pub fn set_usage(&mut self, usage: wgpu::TextureUsages) {
        self.usage = usage;
        self.revision.increase();
    }

    pub fn set_source(&mut self, source: TextureSource) {
        self.source = source;
        self.revision.increase();
    }

    fn create_texture(
        ctx: &TextureCreationContext,
        label: &str,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: &TextureSource,
    ) -> AppResult<wgpu::Texture> {
        let non_srgb_format = format.remove_srgb_suffix();
        let srgb_format = format.add_srgb_suffix();
        let view_formats = if srgb_format != non_srgb_format {
            // Automatically support both srgb-ness views
            vec![non_srgb_format, srgb_format]
        } else {
            vec![]
        };

        let size = match source {
            TextureSource::Dimension(dimension_id) => {
                let dimension_id = dimension_id.ok_or(AppError::UninitializedFields)?;
                let size = ctx.dimensions.get(dimension_id)?.size();

                wgpu::Extent3d {
                    width: size.width(),
                    height: size.height(),
                    depth_or_array_layers: 1,
                }
            }
            TextureSource::Image(dynamic_image) => {
                let (width, height) = dynamic_image.dimensions();
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                }
            }
            TextureSource::Manual { size } => *size,
        };

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &view_formats,
        });

        if let TextureSource::Image(image) = source {
            // TODO: Change this format to an enum that we list all supported formats, instead of relying on all wgpu formats
            match format {
                wgpu::TextureFormat::Rgba32Float => {
                    let rgba = image.to_rgba32f();
                    write_image_to_texture(ctx.queue, &texture, &rgba, size);
                }
                _ => {
                    let rgba = image.to_rgba8();
                    write_image_to_texture(ctx.queue, &texture, &rgba, size);
                }
            }
        }

        Ok(texture)
    }
}

impl TextureRuntime {
    pub fn inner(&self) -> &wgpu::Texture {
        &self.inner
    }
}

impl ProjectResource for Texture {
    type Id = TextureId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl SyncResource for Texture {
    type Context<'a> = TextureCreationContext<'a>;
    type Runtime = TextureRuntime;

    fn sync<'a>(
        &mut self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        let texture =
            Self::create_texture(&ctx, &self.label, self.format, self.usage, &self.source)?;

        Ok(SyncOutcome::Changed(TextureRuntime { inner: texture }))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        match self.source {
            TextureSource::Dimension(Some(dimension_id)) => tracker.was_changed(dimension_id),
            TextureSource::Dimension(None) => false,
            TextureSource::Image(_) | TextureSource::Manual { .. } => false,
        }
    }
}

pub(crate) fn write_image_to_texture<P, Container>(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    image: &image::ImageBuffer<P, Container>,
    size: wgpu::Extent3d,
) where
    P: image::Pixel,
    P::Subpixel: bytemuck::Pod,
    Container: std::ops::Deref<Target = [P::Subpixel]>,
{
    let bytes_per_row =
        image.width() * P::CHANNEL_COUNT as u32 * std::mem::size_of::<P::Subpixel>() as u32;

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            aspect: wgpu::TextureAspect::All,
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
        },
        bytemuck::cast_slice(image.as_raw()),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(size.height),
        },
        size,
    );
}
