use wgpu::util::DeviceExt;

use crate::error::{AppResult, WgpuErrorScope};

pub struct ResizableBuffer {
    buffer: wgpu::Buffer,
}

pub enum ChangeResult {
    Uploaded,
    Recreated,
}

impl ResizableBuffer {
    pub fn new(
        device: &wgpu::Device,
        label: &str,
        usage: wgpu::BufferUsages,
        contents: &[u8],
    ) -> AppResult<Self> {
        let buffer = Self::create_buffer(device, &label, contents, usage)?;
        Ok(Self { buffer })
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> AppResult<ChangeResult> {
        // TODO: recreate buffer if usage or label changes
        if self.buffer.size() != contents.len() as wgpu::BufferAddress {
            self.buffer = Self::create_buffer(device, label, contents, usage)?;
            Ok(ChangeResult::Recreated)
        } else {
            queue.write_buffer(&self.buffer, 0, contents);
            Ok(ChangeResult::Uploaded)
        }
    }

    pub fn inner(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    fn create_buffer(
        device: &wgpu::Device,
        label: &str,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> AppResult<wgpu::Buffer> {
        let scope = WgpuErrorScope::push(device);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        });
        scope.pop()?;
        Ok(buffer)
    }
}
