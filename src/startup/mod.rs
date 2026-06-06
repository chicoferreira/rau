#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
#[cfg(target_arch = "wasm32")]
pub mod url;
