use crate::file;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub viewport: Viewport,
    pub shader: Shader,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Viewport {
    pub clear_color: [f64; 4],
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Shader {
    pub name: String,
    #[serde(flatten)]
    pub shader_type: ShaderType,
}

#[derive(Deserialize, Serialize, Clone)]
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
    pub async fn from_file(path: &str) -> anyhow::Result<Self> {
        let file = file::load_file(path).await?;
        toml::from_str(&file).context("Failed to parse toml file")
    }
}
