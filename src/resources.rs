use std::path::Path;

use anyhow::Context;

use crate::{
    error::{AppError, AppResult},
    project::texture::{Texture, TextureCreationContext, TextureSource},
};

pub async fn load_string(file_name: impl AsRef<Path>) -> anyhow::Result<String> {
    load_binary(file_name)
        .await?
        .try_into()
        .context("Failed to parse UTF-8 string")
}

pub async fn load_binary(file_name: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_arch = "wasm32")]
    let data = {
        let window = web_sys::window().unwrap();
        let location = window.location();
        let mut origin = location.origin().unwrap();
        if !origin.ends_with("res") {
            origin = format!("{}/res", origin);
        }
        let base = reqwest::Url::parse(&format!("{}/", origin)).unwrap();
        let url = base.join(&file_name.as_ref().to_string_lossy()).unwrap();
        reqwest::get(url).await?.bytes().await?.to_vec()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let data = {
        let path = std::path::Path::new(env!("OUT_DIR"))
            .join("res")
            .join(file_name);
        std::fs::read(path)?
    };

    Ok(data)
}

pub async fn load_texture(
    ctx: &TextureCreationContext<'_>,
    file_name: &str,
    format: wgpu::TextureFormat,
) -> AppResult<Texture> {
    let data = load_binary(file_name)
        .await
        .map_err(AppError::FileLoadError)?;
    let img = image::load_from_memory(&data)?;

    let source = TextureSource::Image(img);

    Ok(Texture::new(
        ctx,
        file_name.to_string(),
        format,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        source,
    )?)
}
