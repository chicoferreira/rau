use crate::renderer::camera::Camera;
use wgpu::util::DeviceExt;

#[derive(Default)]
pub struct RenderResourceStorage {
    pub render_bindings: Vec<RenderBinding>,
}

impl RenderResourceStorage {
    pub fn from(bind_groups: impl Into<Vec<RenderBinding>>) -> Self {
        Self {
            render_bindings: bind_groups.into(),
        }
    }

    pub fn upload_camera_uniform(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        for bind_group in &mut self.render_bindings {
            let provider_type = &mut bind_group.provider_type;
            if let BindingResourceType::Uniform(UniformResourceType::Camera(ub)) = provider_type {
                ub.write(queue, CameraUniformData::from_camera(camera));
            }
        }
    }

    pub fn upload_time_delta_uniform(&mut self, queue: &wgpu::Queue, delta: instant::Duration) {
        for bind_group in &mut self.render_bindings {
            let provider_type = &mut bind_group.provider_type;
            if let BindingResourceType::Uniform(UniformResourceType::Time(ub)) = provider_type {
                ub.contents.time += delta.as_secs_f32();
                ub.write(queue, ub.contents);
            }
        }
    }
}

pub struct RenderBinding {
    pub name: String,
    /// The set index of the bind group (in the shader `layout(set = set, binding = …)`)
    pub set: u32,
    pub provider_type: BindingResourceType,
}

pub enum BindingResourceType {
    Uniform(UniformResourceType),
    Texture(usize), // Texture index to the texture vector
}

pub enum UniformResourceType {
    Camera(UniformBuffer<CameraUniformData>),
    Time(UniformBuffer<TimeUniformData>),
    Custom(CustomUniform),
}

impl UniformResourceType {
    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        match self {
            UniformResourceType::Camera(uniform_buffer) => uniform_buffer.get_bind_group(),
            UniformResourceType::Time(uniform_buffer) => uniform_buffer.get_bind_group(),
            UniformResourceType::Custom(uniform_buffer) => match uniform_buffer {
                CustomUniform::Color(uniform_buffer) => uniform_buffer.get_bind_group(),
                CustomUniform::Vec4(uniform_buffer) => uniform_buffer.get_bind_group(),
                CustomUniform::Mat4(uniform_buffer) => uniform_buffer.get_bind_group(),
            },
        }
    }
}

pub enum CustomUniform {
    Color(UniformBuffer<[f32; 4]>),
    Vec4(UniformBuffer<[f32; 4]>),
    Mat4(UniformBuffer<[[f32; 4]; 4]>),
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniformData {
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniformData {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_position: camera.position().to_homogeneous().into(),
            view_proj: camera.calc_matrix().into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TimeUniformData {
    pub time: f32,
    _padding: [u32; 3],
}

impl TimeUniformData {
    pub fn new(time: f32) -> Self {
        Self {
            time,
            _padding: [0; 3],
        }
    }
}

#[derive(Debug)]
pub struct UniformBuffer<T>
where
    T: bytemuck::Pod,
{
    contents: T,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl<T: bytemuck::Pod> UniformBuffer<T> {
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

        UniformBuffer {
            contents,
            buffer,
            bind_group,
        }
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub fn get(&self) -> &T {
        &self.contents
    }

    pub fn write(&mut self, queue: &wgpu::Queue, data: T) {
        self.contents = data;
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[data]));
    }
}
