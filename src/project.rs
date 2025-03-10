use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    pub viewport: Viewport,
    pub shader: Shader,
}

#[derive(Deserialize, Serialize)]
pub struct Viewport {
    pub clear_color: [f64; 4],
}

#[derive(Deserialize, Serialize)]
pub struct Shader {
    pub name: String,
    #[serde(flatten)]
    pub shader_type: ShaderType,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum ShaderType {
    Glsl {
        vertex_shader: PathBuf,
        fragment_shader: PathBuf,
    },
    Wgsl {
        shader: PathBuf,
    },
}

impl Project {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let file = std::fs::read_to_string(path).context("Failed to load file")?;
        toml::from_str(&file).context("Failed to parse toml file")
    }
}
