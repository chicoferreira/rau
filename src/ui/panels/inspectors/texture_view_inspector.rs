use egui::load::SizedTexture;

use crate::{
    project::{TextureViewId, resource::texture_view::TextureViewFormat},
    ui::{
        components::{
            field,
            field_docs::field_doc,
            inspector::{self, AsRichText},
        },
        pane::StateSnapshot,
    },
    utils::texture_format::TextureFormat,
};

impl StateSnapshot<'_> {
    pub fn texture_view_inspector_ui(&mut self, ui: &mut egui::Ui, texture_view_id: TextureViewId) {
        let resolved = self
            .runtime_project
            .texture_views
            .get_init(texture_view_id)
            .ok()
            .flatten()
            .map(|runtime| (runtime.format(), runtime.dimension()));
        let (resolved_format, resolved_dimension) = resolved.unzip();
        let resolved_format = resolved_format.and_then(TextureFormat::from_wgpu);

        let Ok(texture_view) = self.project.texture_views.get_mut(texture_view_id) else {
            ui.label("Texture View couldn't be found.");
            return;
        };

        inspector::section(ui, "Settings", |ui| {
            field::field_grid(ui, "texture_view_inspector_grid", |ui| {
                let mut texture_id = texture_view.texture_id();
                if field::row_doc(
                    ui,
                    "Texture",
                    field_doc!(
                        "The texture this view reads from. A texture view selects a format, \
                        dimension and subresource range of an underlying texture for binding \
                        in shaders.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#gputexture)"
                    ),
                    |ui| {
                        inspector::storage_combo(
                            ui,
                            "texture",
                            &self.project.textures,
                            &mut texture_id,
                        )
                    },
                ) {
                    texture_view.set_texture_id(texture_id);
                }

                const FORMAT_LIST: [Option<TextureViewFormat>; 3] = [
                    None,
                    Some(TextureViewFormat::Srgb),
                    Some(TextureViewFormat::Linear),
                ];

                let mut current_format = texture_view.format();
                let format_changed = field::row_doc(
                    ui,
                    "Format",
                    field_doc!(
                        "Overrides the **format** the texture is interpreted as when sampled.\n\n\
                        - **From Texture**: use the texture's own format.\n\
                        - **Force sRGB / Force Linear**: reinterpret as the sRGB or linear \
                        variant of that format.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gputextureviewdescriptor-format)"
                    ),
                    |ui| {
                        combo_with_resolved(
                            ui,
                            "format",
                            FORMAT_LIST,
                            &mut current_format,
                            resolved_format,
                        )
                    },
                );
                if format_changed {
                    texture_view.set_format(current_format);
                }

                const DIMENSIONS: [Option<wgpu::TextureViewDimension>; 7] = [
                    None,
                    Some(wgpu::TextureViewDimension::D1),
                    Some(wgpu::TextureViewDimension::D2),
                    Some(wgpu::TextureViewDimension::D3),
                    Some(wgpu::TextureViewDimension::D2Array),
                    Some(wgpu::TextureViewDimension::Cube),
                    Some(wgpu::TextureViewDimension::CubeArray),
                ];

                let mut current_dimension = texture_view.dimension();
                let dimension_changed = field::row_doc(
                    ui,
                    "Dimension",
                    field_doc!(
                        "How the texture's data is interpreted as a **view dimension** \
                        (1D, 2D, 2D array, 3D, cube, ...).\n\n\
                        **From Texture** infers it from the underlying texture.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gputextureviewdimension)"
                    ),
                    |ui| {
                        combo_with_resolved(
                            ui,
                            "dimension",
                            DIMENSIONS,
                            &mut current_dimension,
                            resolved_dimension,
                        )
                    },
                );
                if dimension_changed {
                    texture_view.set_dimension(current_dimension);
                }
            });
        });

        inspector::section(ui, "Preview", |ui| {
            let texture_view = match self.runtime_project.texture_views.get_init(texture_view_id) {
                Ok(Some(texture_view)) => texture_view,
                Ok(None) => {
                    ui.spinner();
                    return;
                }
                Err(err) => {
                    field::error_label(ui, err.to_string());
                    return;
                }
            };

            let Some(egui_id) = texture_view.egui_id() else {
                ui.label("Only texture views with Rgba8UnormSrgb format can be previewed.");
                return;
            };

            let size = ui.available_height().min(ui.available_width()).min(500.0);
            let sized_texture = SizedTexture::new(egui_id, (size, size));
            ui.add(egui::Image::new(sized_texture));
        });
    }
}

fn combo_with_resolved<T>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash,
    options: impl IntoIterator<Item = T>,
    current_value: &mut T,
    resolved: Option<impl AsRichText>,
) -> bool
where
    T: AsRichText + Clone + PartialEq,
{
    ui.horizontal(|ui| {
        let changed = inspector::value_combo(ui, id_salt, options, current_value);
        if let Some(resolved) = resolved {
            ui.weak(resolved.as_rich_text());
        }
        changed
    })
    .inner
}

impl AsRichText for Option<TextureViewFormat> {
    fn as_rich_text(&self) -> egui::RichText {
        let r = match self {
            Some(TextureViewFormat::Srgb) => "From Texture (Force sRGB)",
            Some(TextureViewFormat::Linear) => "From Texture (Force Linear)",
            None => "From Texture",
        };
        r.into()
    }
}

impl AsRichText for Option<wgpu::TextureViewDimension> {
    fn as_rich_text(&self) -> egui::RichText {
        match self {
            Some(dimension) => dimension.as_rich_text(),
            None => "From Texture".into(),
        }
        .into()
    }
}
