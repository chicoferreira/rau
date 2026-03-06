use image::GenericImageView;

use crate::project::{
    DimensionId, ViewportId, dimension::Dimension, storage::Storage, viewport::Viewport,
};

pub struct TextureProjectView<'a> {
    pub viewports: &'a Storage<ViewportId, Viewport>,
    pub dimensions: &'a Storage<DimensionId, Dimension>,
}

pub struct Texture {
    pub label: String,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    // TODO: decide if we want that the texture decides which source to grab the size to,
    // or it is the source's job to update the size if it changes
    pub source: TextureSource,
    inner: wgpu::Texture,
}

impl Texture {
    pub fn new(
        project: &TextureProjectView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: String,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: TextureSource,
    ) -> Texture {
        let inner = Self::create_texture(project, device, queue, &label, format, usage, &source);
        Texture {
            label,
            format,
            usage,
            source,
            inner,
        }
    }

    pub fn inner(&self) -> &wgpu::Texture {
        &self.inner
    }

    fn create_texture(
        project: &TextureProjectView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        source: &TextureSource,
    ) -> wgpu::Texture {
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
                let size = project
                    .dimensions
                    .get(*dimension_id)
                    .expect("deal with this later")
                    .size;

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

        let texture = device.create_texture(&wgpu::TextureDescriptor {
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

            queue.write_texture(
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

        texture
    }

    // TODO: Only needs updating when either the source changes size or any parameter updates
}

pub enum TextureSource {
    // Grab size from dimension
    Dimension(DimensionId),
    Image(image::DynamicImage), // TODO: change this to image_id once we have it in the project
    Manual { size: wgpu::Extent3d },
}
