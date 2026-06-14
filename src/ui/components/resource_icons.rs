use egui::{
    Color32, Ui, WidgetText,
    text::{LayoutJob, TextFormat},
};
use egui_phosphor::regular;

use crate::project::{ResourceId, ResourceKind, paths::FilePath};

pub const FOLDER_COLOR: [u8; 3] = [198, 162, 96];

pub fn resource_kind_icon(kind: ResourceKind) -> (&'static str, [u8; 3]) {
    use ResourceKind as K;
    match kind {
        K::Shader => (regular::CODE, [122, 158, 200]),
        K::Viewport => (regular::MONITOR, [120, 180, 195]),
        K::Uniform => (regular::SLIDERS_HORIZONTAL, [122, 190, 170]),
        K::BindGroup => (regular::LINK, [210, 160, 110]),
        K::Texture => (regular::IMAGE, [184, 132, 184]),
        K::TextureView => (regular::EYE, [200, 145, 175]),
        K::Sampler => (regular::EYEDROPPER, [175, 185, 120]),
        K::Dimension => (regular::RULER, [150, 175, 150]),
        K::Camera => (regular::VIDEO_CAMERA, [170, 150, 210]),
        K::Model => (regular::CUBE, [122, 176, 132]),
        K::RenderPipeline => (regular::GRAPH, [200, 130, 130]),
        K::RenderPass => (regular::PAINT_BRUSH, [210, 145, 120]),
        K::ComputePass => (regular::CPU, [150, 150, 215]),
        K::Presentation => (regular::PRESENTATION, [198, 162, 96]),
    }
}

/// Glyph and accent color for a specific resource instance.
pub fn resource_id_icon(id: ResourceId) -> (&'static str, [u8; 3]) {
    resource_kind_icon(resource_id_kind(id))
}

fn resource_id_kind(id: ResourceId) -> ResourceKind {
    use ResourceId as I;
    match id {
        I::Shader(_) => ResourceKind::Shader,
        I::Viewport(_) => ResourceKind::Viewport,
        I::Uniform(_) => ResourceKind::Uniform,
        I::BindGroup(_) => ResourceKind::BindGroup,
        I::Texture(_) => ResourceKind::Texture,
        I::TextureView(_) => ResourceKind::TextureView,
        I::Sampler(_) => ResourceKind::Sampler,
        I::Dimension(_) => ResourceKind::Dimension,
        I::Camera(_) => ResourceKind::Camera,
        I::Model(_) => ResourceKind::Model,
        I::RenderPipeline(_) => ResourceKind::RenderPipeline,
        I::RenderPass(_) => ResourceKind::RenderPass,
        I::Presentation(_) => ResourceKind::Presentation,
        I::ComputePass(_) => ResourceKind::ComputePass,
    }
}

pub fn file_icon(file_path: &FilePath) -> (&'static str, [u8; 3]) {
    let extension = file_path.extension().map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("wgsl" | "glsl" | "vert" | "frag" | "comp") => (regular::CODE, [122, 158, 200]),
        Some("json" | "toml") => (regular::BRACKETS_CURLY, [198, 162, 96]),
        Some("obj" | "gltf" | "glb") => (regular::CUBE, [122, 176, 132]),
        Some("png" | "jpg" | "jpeg" | "hdr") => (regular::IMAGE, [184, 132, 184]),
        _ => (regular::FILE, [150, 150, 150]),
    }
}

pub fn icon_text(ui: &Ui, icon: (&'static str, [u8; 3]), label: &str) -> WidgetText {
    let [r, g, b] = icon.1;
    glyph_text(
        ui,
        icon.0,
        Color32::from_rgb(r, g, b),
        label,
        ui.visuals().text_color(),
    )
}

pub fn warning_text(ui: &Ui, label: &str) -> WidgetText {
    let color = ui.visuals().warn_fg_color;
    glyph_text(ui, regular::WARNING, color, label, color)
}

fn glyph_text(
    ui: &Ui,
    glyph: &str,
    glyph_color: Color32,
    label: &str,
    label_color: Color32,
) -> WidgetText {
    let font_id = egui::TextStyle::Button.resolve(ui.style());

    let mut job = LayoutJob::default();
    job.append(
        glyph,
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: glyph_color,
            ..Default::default()
        },
    );
    job.append(
        label,
        6.0,
        TextFormat {
            font_id,
            color: label_color,
            ..Default::default()
        },
    );
    WidgetText::from(job)
}
