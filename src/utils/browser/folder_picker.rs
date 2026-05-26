//! Browser-only folder import helper.
//!
//! Browsers do not expose native folder paths to wasm. To import a folder we
//! create a hidden `<input type="file" webkitdirectory multiple>`, trigger it
//! from the user action, and then read the returned `FileList`. Each browser
//! `File` carries a `webkitRelativePath` such as `project/assets/model.obj`;
//! this module treats the first path segment as the project name, strips it from
//! every returned `FilePath`, and reads the file bytes with `FileReader`.

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, File, FileList, FileReader, HtmlInputElement,
    js_sys::{Promise, Reflect, Uint8Array},
};

use crate::{
    error::AppResult,
    project::paths::FilePath,
    utils::browser::{browser_error, js_error},
};

pub type PickedFolderFiles = (String, Vec<(FilePath, Vec<u8>)>);

pub async fn pick_folder_files() -> AppResult<Option<PickedFolderFiles>> {
    let Some(files) = pick_directory_file_list().await? else {
        return Ok(None);
    };

    let mut project_name = None;
    let mut result = Vec::new();

    for index in 0..files.length() {
        let file = files
            .get(index)
            .ok_or_else(|| browser_error("selected file was unavailable"))?;

        let relative_path = file_relative_path(&file)?;
        let (folder_name, file_path) = project_file_path(&relative_path)?;
        if project_name.is_none() {
            project_name = Some(folder_name.to_string());
        }

        let bytes = read_web_file(file).await?;
        result.push((file_path, bytes));
    }

    let project_name =
        project_name.ok_or_else(|| browser_error("selected folder did not contain files"))?;

    Ok(Some((project_name, result)))
}

async fn pick_directory_file_list() -> AppResult<Option<FileList>> {
    let window = web_sys::window().ok_or_else(|| browser_error("window unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| browser_error("document unavailable"))?;
    let body = document
        .body()
        .ok_or_else(|| browser_error("document body unavailable"))?;

    let input: HtmlInputElement = document
        .create_element("input")
        .map_err(js_error)?
        .dyn_into()
        .map_err(|_| browser_error("failed to create file input"))?;

    input.set_type("file");
    input.set_multiple(true);
    input.set_webkitdirectory(true);
    input
        .set_attribute("style", "display: none")
        .map_err(js_error)?;
    body.append_child(&input).map_err(js_error)?;

    let promise = Promise::new(&mut |resolve, _reject| {
        let resolve_change = resolve.clone();
        let on_change = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let _ = resolve_change.call0(&JsValue::undefined());
        }));

        input
            .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
            .expect("failed to register folder picker change handler");
        on_change.forget();

        let resolve_cancel = resolve.clone();
        let on_cancel = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let _ = resolve_cancel.call0(&JsValue::undefined());
        }));

        input
            .add_event_listener_with_callback("cancel", on_cancel.as_ref().unchecked_ref())
            .expect("failed to register folder picker cancel handler");
        on_cancel.forget();
    });

    input.click();
    JsFuture::from(promise).await.map_err(js_error)?;
    input.unchecked_ref::<web_sys::Element>().remove();

    Ok(input.files().filter(|files| files.length() > 0))
}

fn file_relative_path(file: &File) -> AppResult<String> {
    let relative_path = Reflect::get(file.as_ref(), &JsValue::from_str("webkitRelativePath"))
        .map_err(js_error)?
        .as_string()
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| file.name());

    Ok(relative_path)
}

fn project_file_path(relative_path: &str) -> AppResult<(&str, FilePath)> {
    let Some((project_name, file_path)) = relative_path
        .split_once('/')
        .or_else(|| relative_path.split_once('\\'))
    else {
        return Err(browser_error(format!(
            "selected file did not include a folder-relative path: {relative_path}"
        )));
    };

    Ok((project_name, FilePath::from_str(file_path)?))
}

async fn read_web_file(file: File) -> AppResult<Vec<u8>> {
    let file_reader = FileReader::new().map_err(js_error)?;

    let promise = Promise::new(&mut |resolve, reject| {
        let reader_for_load = file_reader.clone();
        let resolve_load = resolve.clone();
        let reject_load = reject.clone();
        let on_load =
            Closure::<dyn FnMut()>::wrap(Box::new(move || match reader_for_load.result() {
                Ok(result) => {
                    let _ = resolve_load.call1(&JsValue::undefined(), &result);
                }
                Err(error) => {
                    let _ = reject_load.call1(&JsValue::undefined(), &error);
                }
            }));

        file_reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();

        let reject_error = reject.clone();
        let on_error = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let _ = reject_error.call0(&JsValue::undefined());
        }));

        file_reader.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        on_error.forget();
    });

    file_reader
        .read_as_array_buffer(file.unchecked_ref::<Blob>())
        .map_err(js_error)?;

    let result = JsFuture::from(promise).await.map_err(js_error)?;
    // FileReader returns an ArrayBuffer, which needs copying out of JS memory
    // before it can be stored in IndexedDB through the Rust filesystem layer.
    let buffer = Uint8Array::new(&result);
    let mut bytes = vec![0; buffer.length() as usize];
    buffer.copy_to(&mut bytes);

    Ok(bytes)
}
