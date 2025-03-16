use crate::file;
use anyhow::Context;
use cgmath::{Deg, Point3, Zero};
use default_from_serde::SerdeDefault;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub viewport: Viewport,
    pub shader: Shader,
    #[serde(default)]
    pub camera: Camera,
    #[serde(alias = "model")]
    #[serde(default)]
    pub models: Vec<Model>,
    #[serde(alias = "texture")]
    #[serde(default)]
    pub textures: Vec<Texture>,
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

#[serde_inline_default]
#[derive(Deserialize, Serialize, SerdeDefault, Clone)]
pub struct Camera {
    #[serde_inline_default(Point3::new(0.0, 0.0, 5.0))]
    pub position: Point3<f32>,
    #[serde_inline_default(Deg(-90.0))]
    pub yaw: Deg<f32>,
    #[serde(default = "Deg::zero")]
    pub pitch: Deg<f32>,
    #[serde_inline_default(Deg(60.0))]
    #[serde(alias = "fov")]
    pub fovy: Deg<f32>,
    #[serde_inline_default(0.1)]
    pub znear: f32,
    #[serde_inline_default(100.0)]
    pub zfar: f32,
    #[serde_inline_default(1.5)]
    pub sensitivity: f32,
    #[serde_inline_default(10.0)]
    pub max_speed_per_second: f32,
    #[serde_inline_default(100.0)]
    pub acceleration_per_second: f32,
    #[serde_inline_default(20.0)]
    pub friction_per_second: f32,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Model {
    pub path: PathBuf,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Texture {
    pub path: PathBuf,
}

impl Project {
    pub async fn from_file(path: &str) -> anyhow::Result<Self> {
        let file = file::load_file(path).await?;
        toml::from_str(&file).context("Failed to parse toml file")
    }
}
