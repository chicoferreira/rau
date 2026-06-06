//! Browser-only helpers for reading and rewriting the page URL.

use wasm_bindgen::JsValue;

use crate::{
    error::AppResult,
    utils::browser::{browser_error, js_error},
};

/// Reads the current URL query string (without the leading `?`) and resets the
/// browser URL back to the base path, dropping the query.
///
/// Returns `None` when there is no query to consume, in which case the URL is
/// left untouched.
pub fn take_query_string() -> AppResult<Option<String>> {
    let window = web_sys::window().ok_or_else(|| browser_error("window unavailable"))?;
    let location = window.location();

    let search = location.search().map_err(js_error)?;
    let query = search.trim_start_matches('?');
    if query.is_empty() {
        return Ok(None);
    }
    let query = query.to_string();

    // Reset the browser URL back to the base path so the query parameters are not
    // left around (e.g. on refresh or when sharing the link).
    let history = window.history().map_err(js_error)?;
    let path = location.pathname().map_err(js_error)?;
    history
        .replace_state_with_url(&JsValue::NULL, "", Some(&path))
        .map_err(js_error)?;

    Ok(Some(query))
}
