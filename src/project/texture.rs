use image::GenericImageView;

use crate::{
    error::{AppResult, WgpuErrorScope},
    project::{
        DimensionId, TextureId,
        dimension::Dimension,
        recreate::{ProjectEvent, Recreatable, RecreateTracker},
        storage::Storage,
    },
};

#[derive(Clone, Copy)]
pub struct TextureCreationContext<'a> {
    pub dimensions: &'a Storage<DimensionId, Dimension>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
}

pub struct Texture {
    pub label: String,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    // TODO: decide if we want that the texture decides which source to grab the size to,
    // or it is the source's job to update the size if it changes
    source: TextureSource,
    inner: wgpu::Texture,
    dirty: bool,
    has_error: bool,
}

#[derive(Clone, PartialEq)]
pub enum TextureSource {
    // Grab size from dimension
    Dimension(DimensionId),
    Image(image::DynamicImage), // TODO: change this to image_id once we have it in the project
    Manual { size: wgpu::Extent3d },
}

impl Texture {
    pub fn new(
        project: &TextureCreationContext,
        label: String,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: TextureSource,
    ) -> AppResult<Texture> {
        // TODO: this shoulnd't error on texture creation
        let inner = Self::create_texture(project, &label, format, usage, &source)?;

        Ok(Texture {
            label,
            format,
            usage,
            source,
            inner,
            has_error: false,
            dirty: false,
        })
    }

    pub fn inner(&self) -> &wgpu::Texture {
        &self.inner
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

    pub fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = format;
        self.dirty = true;
    }

    pub fn set_usage(&mut self, usage: wgpu::TextureUsages) {
        self.usage = usage;
        self.dirty = true;
    }

    pub fn set_source(&mut self, source: TextureSource) {
        self.source = source;
        self.dirty = true;
    }

    fn create_texture(
        ctx: &TextureCreationContext,
        label: &str,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: &TextureSource,
    ) -> AppResult<wgpu::Texture> {
        let scope = WgpuErrorScope::push(ctx.device);
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
                let size = ctx.dimensions.get(*dimension_id)?.size;

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
            let rgba = image.to_rgba8();

            ctx.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    aspect: wgpu::TextureAspect::All,
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                &rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * size.width),
                    rows_per_image: Some(size.height),
                },
                size,
            );
        }

        scope.pop()?;

        Ok(texture)
    }
}

impl Recreatable for Texture {
    type Context<'a> = TextureCreationContext<'a>;
    type Id = TextureId;

    fn recreate<'a>(
        &mut self,
        id: Self::Id,
        ctx: &mut Self::Context<'a>,
        _tracker: &RecreateTracker,
    ) -> AppResult<Option<ProjectEvent>> {
        let dirty_source = {
            let mut result = false;
            match &self.source {
                TextureSource::Dimension(dimension_id) => {
                    if let Ok(dimension) = ctx.dimensions.get(*dimension_id) {
                        let current_size = self.inner.size();
                        result = dimension.size.width() != current_size.width
                            || dimension.size.height() != current_size.height;
                    }
                }
                _ => (),
            }
            result
        };

        if !self.dirty && !self.has_error && !dirty_source {
            return Ok(None);
        }

        self.inner = Self::create_texture(&ctx, &self.label, self.format, self.usage, &self.source)
            .inspect_err(|_| self.has_error = true)?;

        self.has_error = false;
        self.dirty = false;

        Ok(Some(ProjectEvent::TextureRecreated(id)))
    }
}
