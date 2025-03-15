use crate::project::ShaderType;
use crate::{file, project};

pub enum Shader {
    Wgsl(wgpu::ShaderModule),
    Glsl(wgpu::ShaderModule, wgpu::ShaderModule),
}

impl Shader {
    pub async fn load(device: &wgpu::Device, shader: &project::Shader) -> anyhow::Result<Self> {
        match &shader.shader_type {
            ShaderType::Glsl {
                vertex_shader,
                fragment_shader,
            } => {
                let vertex_shader = file::load_file(vertex_shader).await?;
                let fragment_shader = file::load_file(fragment_shader).await?;

                let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&format!("{} Vertex Shader", shader.name)),
                    source: wgpu::ShaderSource::Glsl {
                        shader: vertex_shader.into(),
                        stage: wgpu::naga::ShaderStage::Vertex,
                        defines: Default::default(),
                    },
                });

                let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&format!("{} Fragment Shader", shader.name)),
                    source: wgpu::ShaderSource::Glsl {
                        shader: fragment_shader.into(),
                        stage: wgpu::naga::ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                });

                Ok(Shader::Glsl(vertex_shader, fragment_shader))
            }
            ShaderType::Wgsl {
                shader: shader_path,
            } => {
                let shader_content = file::load_file(shader_path).await?;

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(&format!("{} Shader", shader.name)),
                    source: wgpu::ShaderSource::Wgsl(shader_content.into()),
                });

                Ok(Shader::Wgsl(shader))
            }
        }
    }

    pub fn vertex(&self) -> &wgpu::ShaderModule {
        match self {
            Shader::Wgsl(shader) => shader,
            Shader::Glsl(vertex, _) => vertex,
        }
    }

    pub fn fragment(&self) -> &wgpu::ShaderModule {
        match self {
            Shader::Wgsl(shader) => shader,
            Shader::Glsl(_, fragment) => fragment,
        }
    }
}
