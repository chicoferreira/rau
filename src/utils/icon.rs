use winit::window::WindowAttributes;

pub fn apply_icon(attributes: WindowAttributes) -> WindowAttributes {
    let image_bytes = include_bytes!("../../assets/rau-app-icon.png");
    let image = image::load_from_memory(image_bytes)
        .expect("Failed to decode app window icon")
        .into_rgba8();
    let (width, height) = image.dimensions();

    let icon = winit::window::Icon::from_rgba(image.into_raw(), width, height)
        .expect("Failed to create app window icon");

    attributes.with_window_icon(Some(icon))
}
