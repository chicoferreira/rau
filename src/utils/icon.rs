pub const LOGO_IMAGE_BYTES: &[u8] = include_bytes!("../../assets/rau-app-icon.png");

#[cfg(not(target_arch = "wasm32"))]
pub fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(LOGO_IMAGE_BYTES)
        .expect("Failed to decode app window icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}
