use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub async fn load_file(file: impl AsRef<Path>) -> anyhow::Result<String> {
    use anyhow::Context;
    std::fs::read_to_string(file).context("Failed to load file")
}

#[cfg(target_arch = "wasm32")]
pub async fn load_file(file: impl AsRef<Path>) -> anyhow::Result<String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, window};

    let url = file.as_ref();
    let window = window().ok_or_else(|| anyhow::anyhow!("No global `window` exists"))?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url.to_str().unwrap(), &opts)
        .map_err(|err| anyhow::anyhow!("Failed to create request: {:?}", err))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|err| anyhow::anyhow!("Failed to execute request: {:?}", err))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| anyhow::anyhow!("Failed to convert to Response"))?;

    let result = resp.text().map_err(|err| anyhow::anyhow!("{:?}", err))?;
    let text_value = JsFuture::from(result)
        .await
        .map_err(|err| anyhow::anyhow!("{:?}", err))?;

    text_value
        .as_string()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert response to string"))
}
