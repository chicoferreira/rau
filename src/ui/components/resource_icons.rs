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

    fn into_text(self, font: FontId, label: &str, label_color: Color32, gap: f32) -> WidgetText {
        glyph_text(font, self.glyph, self.color, label, label_color, gap)
    }
}

pub const FOLDER_COLOR: Color32 = Color32::from_rgb(226, 170, 68);

pub fn resource_kind_icon(kind: ResourceKind) -> Icon {
    use ResourceKind as K;
    match kind {
        K::Shader => Icon::new(regular::CODE, [97, 154, 221]),
        K::Viewport => Icon::new(regular::MONITOR, [97, 192, 215]),
        K::Uniform => Icon::new(regular::SLIDERS_HORIZONTAL, [102, 208, 177]),
        K::BindGroup => Icon::new(regular::LINK, [237, 158, 79]),
        K::Texture => Icon::new(regular::IMAGE, [197, 116, 197]),
        K::TextureView => Icon::new(regular::EYE, [212, 121, 171]),
        K::Sampler => Icon::new(regular::EYEDROPPER, [187, 203, 102]),
        K::Dimension => Icon::new(regular::RULER, [140, 180, 140]),
        K::Camera => Icon::new(regular::VIDEO_CAMERA, [155, 120, 223]),
        K::Model => Icon::new(regular::CUBE, [107, 191, 123]),
        K::RenderPipeline => Icon::new(regular::GRAPH, [218, 105, 105]),
        K::RenderPass => Icon::new(regular::PAINT_BRUSH, [234, 129, 89]),
        K::ComputePass => Icon::new(regular::CPU, [117, 117, 230]),
        K::Presentation => Icon::new(regular::PRESENTATION, [226, 170, 68]),
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
            Icon::new(regular::CODE, [97, 154, 221])
        }
        Some("json" | "toml") => Icon::new(regular::BRACKETS_CURLY, [226, 170, 68]),
        Some("obj" | "gltf" | "glb") => Icon::new(regular::CUBE, [107, 191, 123]),
        Some("png" | "jpg" | "jpeg" | "hdr") => Icon::new(regular::IMAGE, [197, 116, 197]),
        _ => Icon::new(regular::FILE, [150, 150, 150]),
    }
}

/// Default horizontal spacing between a glyph and its label.
const DEFAULT_GLYPH_GAP: f32 = 6.0;

/// `icon + label` using the button font, with the label in the default text color.
pub fn icon_text(ui: &Ui, icon: Icon, label: &str) -> WidgetText {
    icon.into_text(
        button_font(ui),
        label,
        ui.visuals().text_color(),
        DEFAULT_GLYPH_GAP,
    )
}

/// `icon + label` sized for tab titles, with the label in the placeholder color.
pub fn icon_tab_title(icon: Icon, label: &str) -> WidgetText {
    icon.into_text(
        FontId::proportional(13.0),
        label,
        Color32::PLACEHOLDER,
        DEFAULT_GLYPH_GAP,
    )
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

pub fn drag_handle_text(ui: &Ui, label: &str) -> WidgetText {
    glyph_text(
        button_font(ui),
        regular::DOTS_SIX_VERTICAL,
        ui.visuals().weak_text_color(),
        label,
        ui.visuals().text_color(),
        2.0,
    )
}

/// `glyph + label` in the button font, with both drawn in the same `color`.
fn mono_text(ui: &Ui, glyph: &'static str, color: Color32, label: &str) -> WidgetText {
    Icon { glyph, color }.into_text(button_font(ui), label, color, DEFAULT_GLYPH_GAP)
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
    gap: f32,
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
        gap,
        TextFormat {
            font_id,
            color: label_color,
            ..Default::default()
        },
    );
    WidgetText::from(job)
}
