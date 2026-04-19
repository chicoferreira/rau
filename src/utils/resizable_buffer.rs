use wgpu::util::DeviceExt;

pub struct ResizableBuffer {
    label: String,
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
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
    ) -> Self {
        let buffer = Self::create_buffer(device, label, contents, usage);
        Self {
            label: label.to_string(),
            buffer,
            usage,
        }
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        contents: &[u8],
        usage: wgpu::BufferUsages,
    ) -> ChangeResult {
        if self.buffer.size() != contents.len() as wgpu::BufferAddress
            || self.usage != usage
            || self.label != label
        {
            self.buffer = Self::create_buffer(device, label, contents, usage);
            ChangeResult::Recreated
        } else {
            queue.write_buffer(&self.buffer, 0, contents);
            ChangeResult::Uploaded
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
    ) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        })
    }
}
