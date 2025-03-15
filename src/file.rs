use anyhow::Context;
use std::path::Path;

pub async fn load_file(file: impl AsRef<Path>) -> anyhow::Result<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = file.as_ref();
        std::fs::read_to_string(file).context(format!("Failed to load file: {}", file.display()))
    }

    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file.as_ref());
        Ok(reqwest::get(url)
            .await
            .context("Failed to fetch file")?
            .text()
            .await
            .context("Failed to read text")?)
    }
}

pub async fn load_file_bytes(file: impl AsRef<Path>) -> anyhow::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = file.as_ref();
        std::fs::read(file).context(format!("Failed to load file: {}", file.display()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file.as_ref());
        Ok(reqwest::get(url)
            .await
            .context("Failed to fetch file")?
            .bytes()
            .await
            .context("Failed to read bytes")?
            .to_vec())
    }
}

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &Path) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let origin = location.origin().unwrap();
    let base = reqwest::Url::parse(&format!("{}/", origin)).unwrap();
    base.join(file_name.to_string_lossy().as_ref()).unwrap()
}
