//! Reads a rendered texture back from the GPU and saves it as a PNG.
//!
//! The GPU copy and submit happen synchronously on the render thread (where the
//! device and queue live), and the rest (mapping the buffer, encoding the PNG and
//! prompting the user for a save location) runs as an [`AsyncJob`] so it doesn't
//! block the frame.

use std::{io::Cursor, task::Poll};

use image::ImageEncoder;

use crate::{
    error::{AppError, AppResult},
    project::{Project, ProjectResource, RuntimeProject, TextureId},
    utils::{async_job::AsyncJob, background_task, wgpu_utils::create_command_encoder},
};

/// Owns the queue of requested texture image downloads and their in-flight save jobs.
///
/// A request is queued by the UI and only turned into GPU work later, once the caller has the
/// device, queue and a freshly rendered texture available (see [`TextureCaptures::tick`]).
#[derive(Default)]
pub struct TextureCaptures {
    /// Textures whose image the user requested to download, not yet started.
    pending: Vec<TextureId>,
    /// In-flight captures (GPU readback, PNG encoding and the save prompt).
    tasks: Vec<AsyncJob<AppResult<()>>>,
}

impl TextureCaptures {
    pub fn request(&mut self, texture_id: TextureId) {
        self.pending.push(texture_id);
    }

    pub fn tick(
        &mut self,
        project: &Project,
        runtime_project: &RuntimeProject,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        toasts: &mut egui_notify::Toasts,
    ) {
        for texture_id in std::mem::take(&mut self.pending) {
            match start_capture(project, runtime_project, device, queue, texture_id) {
                Ok(task) => self.tasks.push(task),
                Err(error) => {
                    toasts_log_error!(toasts, "Failed to capture texture image: {error}");
                }
            }
        }

        self.tasks.retain_mut(|task| match task.try_resolve() {
            Poll::Ready(Ok(())) => false,
            Poll::Ready(Err(error)) => {
                toasts_log_error!(toasts, "Failed to save texture image: {error}");
                false
            }
            Poll::Pending => true,
        });
    }
}

fn start_capture(
    project: &Project,
    runtime_project: &RuntimeProject,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture_id: TextureId,
) -> AppResult<AsyncJob<AppResult<()>>> {
    let texture = project.textures.get(texture_id)?;

    let runtime_texture = runtime_project
        .textures
        .get_init(texture_id)?
        .ok_or(AppError::CaptureError("Texture is not ready yet".into()))?;

    let file_name = format!("{}.png", texture.label());

    download_texture_as_png(device, queue, runtime_texture.inner(), file_name)
}

pub fn download_texture_as_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    file_name: String,
) -> AppResult<AsyncJob<AppResult<()>>> {
    if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
        return Err(AppError::CaptureError(
            "the texture needs the 'Copy Source' usage to be saved as an image".into(),
        ));
    }

    let format = texture.format();
    let block_size = format
        .block_copy_size(None)
        .ok_or(AppError::CaptureError(format!(
            "texture format {format:?} cannot be copied"
        )))?;

    let color_type = png_color_type(format)?;

    let width = texture.width();
    let height = texture.height();
    if width == 0 || height == 0 {
        return Err(AppError::CaptureError(
            "Texture height/width cannot be zero".into(),
        ));
    }

    let unpadded_bytes_per_row = width * block_size;
    let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Texture Capture Readback Buffer"),
        size: (padded_bytes_per_row as u64) * (height as u64),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = create_command_encoder(device, "Texture Capture Encoder");
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);

    let device = device.clone();
    let readback = background_task::spawn_future("texture-capture-readback", async move {
        let pixels = read_buffer(
            &device,
            &buffer,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        )
        .await?;
        encode_png(&pixels, width, height, color_type)
    });

    Ok(AsyncJob::new(async move {
        let png = readback.await?;
        save_png(file_name, png).await
    }))
}

async fn read_buffer(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    height: u32,
    padded_bytes_per_row: u32,
    unpadded_bytes_per_row: u32,
) -> AppResult<Vec<u8>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|err| AppError::CaptureError(format!("device poll failed: {err}")))?;

    receiver
        .await
        .map_err(|_| AppError::CaptureError("buffer map callback was dropped".into()))?
        .map_err(|err| AppError::CaptureError(format!("buffer map failed: {err}")))?;

    let mapped = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((unpadded_bytes_per_row as usize) * (height as usize));
    for row in 0..height {
        let start = (row * padded_bytes_per_row) as usize;
        let end = start + unpadded_bytes_per_row as usize;
        pixels.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    buffer.unmap();

    Ok(pixels)
}

fn png_color_type(format: wgpu::TextureFormat) -> AppResult<image::ExtendedColorType> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
            Ok(image::ExtendedColorType::Rgba8)
        }
        other => Err(AppError::CaptureError(format!(
            "Texture format {other:?} cannot be saved as a PNG image"
        ))),
    }
}

fn encode_png(
    pixels: &[u8],
    width: u32,
    height: u32,
    color_type: image::ExtendedColorType,
) -> AppResult<Vec<u8>> {
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(Cursor::new(&mut png));
    encoder.write_image(pixels, width, height, color_type)?;
    Ok(png)
}

async fn save_png(file_name: String, png: Vec<u8>) -> AppResult<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(handle) = rfd::AsyncFileDialog::new()
            .set_title("Save Texture Image")
            .set_file_name(&file_name)
            .add_filter("PNG Image", &["png"])
            .save_file()
            .await
        else {
            return Ok(());
        };

        handle.write(&png).await?;
        Ok(())
    }
    #[cfg(target_arch = "wasm32")]
    {
        // rfd is creating a div with an anchor link instead of downloading the file directly
        crate::utils::browser::download_file(&file_name, png)
    }
}
