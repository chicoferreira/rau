use egui::{
    Color32, FontId, Ui, WidgetText,
    text::{LayoutJob, TextFormat},
};
use egui_phosphor::regular;

use crate::project::{ResourceId, ResourceKind, paths::FilePath};

/// A phosphor glyph paired with an accent color.
#[derive(Clone, Copy)]
pub struct Icon {
    pub glyph: &'static str,
    pub color: Color32,
}

impl Icon {
    pub const fn new(glyph: &'static str, [r, g, b]: [u8; 3]) -> Self {
        Self {
            glyph,
            color: Color32::from_rgb(r, g, b),
        }
    }

    /// `glyph + label`, with the glyph in the icon's accent color and the label
    /// in `label_color`.
    fn into_text(self, font: FontId, label: &str, label_color: Color32) -> WidgetText {
        glyph_text(font, self.glyph, self.color, label, label_color)
    }
}

pub const FOLDER_COLOR: Color32 = Color32::from_rgb(198, 162, 96);

pub fn resource_kind_icon(kind: ResourceKind) -> Icon {
    use ResourceKind as K;
    match kind {
        K::Shader => Icon::new(regular::CODE, [122, 158, 200]),
        K::Viewport => Icon::new(regular::MONITOR, [120, 180, 195]),
        K::Uniform => Icon::new(regular::SLIDERS_HORIZONTAL, [122, 190, 170]),
        K::BindGroup => Icon::new(regular::LINK, [210, 160, 110]),
        K::Texture => Icon::new(regular::IMAGE, [184, 132, 184]),
        K::TextureView => Icon::new(regular::EYE, [200, 145, 175]),
        K::Sampler => Icon::new(regular::EYEDROPPER, [175, 185, 120]),
        K::Dimension => Icon::new(regular::RULER, [150, 175, 150]),
        K::Camera => Icon::new(regular::VIDEO_CAMERA, [170, 150, 210]),
        K::Model => Icon::new(regular::CUBE, [122, 176, 132]),
        K::RenderPipeline => Icon::new(regular::GRAPH, [200, 130, 130]),
        K::RenderPass => Icon::new(regular::PAINT_BRUSH, [210, 145, 120]),
        K::ComputePass => Icon::new(regular::CPU, [150, 150, 215]),
        K::Presentation => Icon::new(regular::PRESENTATION, [198, 162, 96]),
    }
}

/// Glyph and accent color for a specific resource instance.
pub fn resource_id_icon(id: ResourceId) -> Icon {
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

pub fn file_icon(file_path: &FilePath) -> Icon {
    let extension = file_path.extension().map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("wgsl" | "glsl" | "vert" | "frag" | "comp") => {
            Icon::new(regular::CODE, [122, 158, 200])
        }
        Some("json" | "toml") => Icon::new(regular::BRACKETS_CURLY, [198, 162, 96]),
        Some("obj" | "gltf" | "glb") => Icon::new(regular::CUBE, [122, 176, 132]),
        Some("png" | "jpg" | "jpeg" | "hdr") => Icon::new(regular::IMAGE, [184, 132, 184]),
        _ => Icon::new(regular::FILE, [150, 150, 150]),
    }
}

/// `icon + label` using the button font, with the label in the default text color.
pub fn icon_text(ui: &Ui, icon: Icon, label: &str) -> WidgetText {
    icon.into_text(button_font(ui), label, ui.visuals().text_color())
}

/// `icon + label` sized for tab titles, with the label in the placeholder color.
pub fn icon_tab_title(icon: Icon, label: &str) -> WidgetText {
    icon.into_text(FontId::proportional(13.0), label, Color32::PLACEHOLDER)
}

/// Button label prefixed with a "plus" glyph, for actions that add a new item
/// (bind group entry, uniform field, attribute, …).
pub fn add_text(ui: &Ui, label: &str) -> WidgetText {
    mono_text(ui, regular::PLUS, ui.visuals().text_color(), label)
}

/// Button label prefixed with a "magic wand" glyph, for actions that derive a
/// resource from existing data (texture view, bind groups from materials, …).
pub fn derive_text(ui: &Ui, label: &str) -> WidgetText {
    mono_text(ui, regular::MAGIC_WAND, ui.visuals().text_color(), label)
}

pub fn warning_text(ui: &Ui, label: &str) -> WidgetText {
    mono_text(ui, regular::WARNING, ui.visuals().warn_fg_color, label)
}

/// `glyph + label` in the button font, with both drawn in the same `color`.
fn mono_text(ui: &Ui, glyph: &'static str, color: Color32, label: &str) -> WidgetText {
    Icon { glyph, color }.into_text(button_font(ui), label, color)
}

fn button_font(ui: &Ui) -> FontId {
    egui::TextStyle::Button.resolve(ui.style())
}

fn glyph_text(
    font_id: FontId,
    glyph: &str,
    glyph_color: Color32,
    label: &str,
    label_color: Color32,
) -> WidgetText {
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
