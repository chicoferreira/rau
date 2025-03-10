use crate::project;
use crate::project::ShaderType;
use anyhow::Context;
use std::path::Path;

pub enum Shader {
    Wgsl(wgpu::ShaderModule),
    Glsl(wgpu::ShaderModule, wgpu::ShaderModule),
}

impl Shader {
    pub fn load(device: &wgpu::Device, shader: &project::Shader) -> anyhow::Result<Self> {
        fn load_file(file: impl AsRef<Path>) -> anyhow::Result<String> {
            std::fs::read_to_string(file).context("Failed to load file")
        }

        match &shader.shader_type {
            ShaderType::Glsl {
                vertex_shader,
                fragment_shader,
            } => {
                let vertex_shader = load_file(vertex_shader)?;
                let fragment_shader = load_file(fragment_shader)?;

                let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Vertex Shader"),
                    source: wgpu::ShaderSource::Glsl {
                        shader: vertex_shader.into(),
                        stage: wgpu::naga::ShaderStage::Vertex,
                        defines: Default::default(),
                    },
                });

                let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Fragment Shader"),
                    source: wgpu::ShaderSource::Glsl {
                        shader: fragment_shader.into(),
                        stage: wgpu::naga::ShaderStage::Fragment,
                        defines: Default::default(),
                    },
                });

                Ok(Shader::Glsl(vertex_shader, fragment_shader))
            }
            ShaderType::Wgsl { shader } => {
                let shader = load_file(shader)?;

                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Shader"),
                    source: wgpu::ShaderSource::Wgsl(shader.into()),
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
