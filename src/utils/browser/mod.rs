pub mod file_download;
pub mod folder_picker;
pub mod url;

use wasm_bindgen::JsValue;
use web_sys::js_sys::Reflect;

use crate::error::AppError;

pub(super) fn browser_error(message: impl Into<String>) -> AppError {
    AppError::BrowserError(message.into())
}

pub(super) fn js_error(error: JsValue) -> AppError {
    let message = error
        .as_string()
        .or_else(|| {
            Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{error:?}"));

    browser_error(message)
}
