use crate::file;
use anyhow::Context;
use cgmath::{Deg, Point3, Zero};
use default_from_serde::SerdeDefault;
use serde::{Deserialize, Serialize};
use serde_inline_default::serde_inline_default;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Clone)]
pub struct Project {
    pub name: String,
    pub viewport: Viewport,
    #[serde(alias = "shader")]
    pub shaders: Vec<Shader>,
    #[serde(default)]
    pub camera: Camera,
    #[serde(alias = "model")]
    #[serde(default)]
    pub models: Vec<Model>,
    #[serde(alias = "texture")]
    #[serde(default)]
    pub textures: Vec<Texture>,
    #[serde(alias = "render_pipeline")]
    pub render_pipeline: RenderPipeline,
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
    #[serde(default)]
    pub name: Option<String>,
    pub path: PathBuf,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct RenderPipeline {
    pub name: String,
    #[serde(flatten)]
    pub shader: ShaderIdentifier,
    #[serde(alias = "bind_group")]
    pub bind_groups: HashMap<String, BindGroupIdentifier>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ShaderIdentifier {
    pub shader_name: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BindGroupIdentifier {
    #[serde(alias = "set")]
    pub index: u32,
    #[serde(flatten)]
    pub bind_group_type: BindGroupIdentifierType,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum BindGroupIdentifierType {
    Camera,
    Texture { texture_name: String },
    Time,
    Custom(CustomUniformType),
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "custom_type")]
#[serde(rename_all = "lowercase")]
pub enum CustomUniformType {
    Color,
    Vec4,
    Mat4,
}

impl Project {
    pub async fn from_file(path: &str) -> anyhow::Result<Self> {
        let file = file::load_file(path).await?;
        toml::from_str(&file).context("Failed to parse toml file")
    }
}
