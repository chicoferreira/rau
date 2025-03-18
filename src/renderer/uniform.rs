use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct GpuUniform<T>
where
    T: bytemuck::Pod,
{
    _phantom: std::marker::PhantomData<T>,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl<T: bytemuck::Pod> GpuUniform<T> {
    pub fn new(
        device: &wgpu::Device,
        contents: T,
        bind_group_layout: &wgpu::BindGroupLayout,
        binding: u32,
        label: Option<&str>,
    ) -> Self {
        let buffer_label = label.map(|l| format!("{} Buffer", l));
        let bind_group_label = label.map(|l| format!("{} Bind Group", l));

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: buffer_label.as_deref(),
            contents: bytemuck::cast_slice(&[contents]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding,
                resource: buffer.as_entire_binding(),
            }],
            label: bind_group_label.as_deref(),
        });

        GpuUniform {
            _phantom: std::marker::PhantomData,
            buffer,
            bind_group,
        }
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn write_to_queue(&self, queue: &wgpu::Queue, data: T) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }
}

pub trait GpuUniformAcessor {
    fn get_bind_group(&self) -> &wgpu::BindGroup;
    fn upload_gpu_uniform(&mut self, queue: &wgpu::Queue);
}

pub mod time {
    use crate::renderer::uniform::GpuUniform;

    #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    #[repr(C)]
    pub struct Time {
        pub time: f32,
        _padding: [u32; 3],
    }

    pub struct TimeUniform {
        pub gpu_uniform: GpuUniform<Time>,
    }

    impl TimeUniform {
        pub fn new(
            device: &wgpu::Device,
            bind_group_layout: &wgpu::BindGroupLayout,
            binding: u32,
            label: Option<&str>,
        ) -> Self {
            let gpu_uniform = GpuUniform::new(
                device,
                Time { time: 0.0, _padding: [0; 3] },
                bind_group_layout,
                binding,
                label,
            );
            Self { gpu_uniform }
        }

        pub fn update_time(&mut self, queue: &wgpu::Queue, time: f32) {
            self.gpu_uniform.write_to_queue(queue, Time { time, _padding: [0; 3] });
        }
    }
}
