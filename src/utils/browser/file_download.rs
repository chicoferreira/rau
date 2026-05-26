//! Browser-only file download helper.

use wasm_bindgen::JsCast;
use web_sys::{
    Blob, HtmlElement, Url,
    js_sys::{Array, Uint8Array},
};

use crate::{
    error::AppResult,
    utils::browser::{browser_error, js_error},
};

pub fn download_file(file_name: &str, bytes: Vec<u8>) -> AppResult<()> {
    let window = web_sys::window().ok_or_else(|| browser_error("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| browser_error("document unavailable"))?;
    let body = document
        .body()
        .ok_or_else(|| browser_error("document body unavailable"))?;

    let bytes = Uint8Array::from(bytes.as_slice());
    let blob_parts = Array::new();
    blob_parts.push(&bytes);
    let blob = Blob::new_with_u8_array_sequence(&blob_parts).map_err(js_error)?;
    let object_url = Url::create_object_url_with_blob(&blob).map_err(js_error)?;

    let result = (|| {
        let anchor = document.create_element("a").map_err(js_error)?;
        anchor
            .set_attribute("href", &object_url)
            .map_err(js_error)?;
        anchor
            .set_attribute("download", file_name)
            .map_err(js_error)?;
        anchor
            .set_attribute("style", "display: none")
            .map_err(js_error)?;

        body.append_child(&anchor).map_err(js_error)?;
        anchor.unchecked_ref::<HtmlElement>().click();
        anchor.remove();

        Ok(())
    })();

    Url::revoke_object_url(&object_url).map_err(js_error)?;
    result
}
