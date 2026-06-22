use winit::window::WindowAttributes;

pub const LOGO_IMAGE_BYTES: &[u8] = include_bytes!("../../assets/rau-app-icon.png");

pub fn apply_icon(attributes: WindowAttributes) -> WindowAttributes {
    let image = image::load_from_memory(LOGO_IMAGE_BYTES)
        .expect("Failed to decode app window icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    let icon = winit::window::Icon::from_rgba(image.into_raw(), width, height)
        .expect("Failed to create app window icon");

    attributes.with_window_icon(Some(icon))
}
