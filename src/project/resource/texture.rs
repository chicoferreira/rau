use image::GenericImageView;
use serde::{Deserialize, Serialize};
use std::task::Poll;

use crate::{
    error::{AppError, AppResult},
    file::file_storage::FileStorage,
    project::{
        DimensionId, ProjectResource, TextureId,
        paths::FilePath,
        resource::dimension::Dimension,
        storage::Storage,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    utils::{async_job::AsyncJob, wgpu_error_scope::WgpuErrorScope},
};

#[derive(Clone, Copy)]
pub struct TextureCreationContext<'a> {
    pub dimensions: &'a Storage<Dimension>,
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub file_storage: &'a FileStorage,
    pub downlevel_flags: wgpu::DownlevelFlags,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Texture {
    label: String,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    source: TextureSource,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

pub struct TextureRuntime {
    inner: wgpu::Texture,
}

#[derive(Default)]
pub enum TextureJob {
    #[default]
    Start,
    ReadingImage(AsyncJob<AppResult<Vec<u8>>>),
    Validation(TextureRuntime, AsyncJob<AppResult<()>>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum TextureSource {
    // Grab size from dimension
    Dimension(Option<DimensionId>),
    Image(FilePath),
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
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
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
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn set_format(&mut self, format: wgpu::TextureFormat) {
        self.format = format;
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn set_usage(&mut self, usage: wgpu::TextureUsages) {
        self.usage = usage;
        self.runtime_revision.increase();
        self.project_revision.increase();
    }

    pub fn set_source(&mut self, source: TextureSource) {
        self.source = source;
        self.runtime_revision.increase();
        self.project_revision.increase();
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

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for Texture {
    type Context<'a> = TextureCreationContext<'a>;
    type Runtime = TextureRuntime;
    type Job = TextureJob;

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        let image_bytes = match job {
            TextureJob::Start => None,
            TextureJob::ReadingImage(mut future) => match future.try_resolve() {
                Poll::Ready(result) => Some(result?),
                Poll::Pending => {
                    return Ok(SyncOutcome::Pending(TextureJob::ReadingImage(future)));
                }
            },
            TextureJob::Validation(runtime, mut future) => {
                return match future.try_resolve() {
                    Poll::Ready(result) => result.map(|()| SyncOutcome::Changed(runtime)),
                    Poll::Pending => Ok(SyncOutcome::Pending(TextureJob::Validation(
                        runtime, future,
                    ))),
                };
            }
        };

        let non_srgb_format = self.format.remove_srgb_suffix();
        let srgb_format = self.format.add_srgb_suffix();
        let supports_view_formats = ctx
            .downlevel_flags
            .contains(wgpu::DownlevelFlags::VIEW_FORMATS);
        let view_formats = if supports_view_formats && srgb_format != non_srgb_format {
            // Automatically support both srgb-ness views
            vec![non_srgb_format, srgb_format]
        } else {
            vec![]
        };

        let mut image_to_write = None;

        let size = match &self.source {
            TextureSource::Dimension(dimension_id) => {
                let dimension_id = dimension_id.ok_or(AppError::UninitializedFields)?;
                let size = ctx.dimensions.get(dimension_id)?.size();

                wgpu::Extent3d {
                    width: size.width(),
                    height: size.height(),
                    depth_or_array_layers: 1,
                }
            }
            TextureSource::Image(path) => {
                let bytes = match image_bytes {
                    Some(bytes) => bytes,
                    None => {
                        return Ok(SyncOutcome::Pending(TextureJob::ReadingImage(
                            ctx.file_storage.read(path),
                        )));
                    }
                };
                let dynamic_image = image::load_from_memory(&bytes)?;

                let (width, height) = dynamic_image.dimensions();

                image_to_write = Some(dynamic_image);

                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                }
            }
            TextureSource::Manual { size } => *size,
        };

        let scope = WgpuErrorScope::push(ctx.device);

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&self.label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: self.usage,
            view_formats: &view_formats,
        });

        if let Some(image_to_write) = image_to_write {
            // TODO: Change this format to an enum that we list all supported formats, instead of relying on all wgpu formats
            match self.format {
                wgpu::TextureFormat::Rgba32Float => {
                    let rgba = image_to_write.to_rgba32f();
                    write_image_to_texture(ctx.queue, &texture, &rgba, size);
                }
                _ => {
                    let rgba = image_to_write.to_rgba8();
                    write_image_to_texture(ctx.queue, &texture, &rgba, size);
                }
            }
        }

        let runtime = TextureRuntime { inner: texture };
        Ok(SyncOutcome::Pending(TextureJob::Validation(
            runtime,
            scope.pop(),
        )))
    }

    fn needs_rebuild_from_others(&self, tracker: &SyncTracker) -> bool {
        match &self.source {
            TextureSource::Dimension(Some(dimension_id)) => tracker.was_changed(*dimension_id),
            TextureSource::Dimension(None) => false,
            TextureSource::Image(path) => tracker.file_changed(&path),
            TextureSource::Manual { .. } => false,
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
