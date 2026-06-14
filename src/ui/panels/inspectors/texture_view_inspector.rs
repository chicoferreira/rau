use egui::{Grid, load::SizedTexture};

use crate::{
    project::{TextureViewId, resource::texture_view::TextureViewFormat},
    ui::{
        components::inspector::{self, AsWidgetText},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn texture_view_inspector_ui(&mut self, ui: &mut egui::Ui, texture_view_id: TextureViewId) {
        let Ok(texture_view) = self.project.texture_views.get_mut(texture_view_id) else {
            ui.label("Texture View couldn't be found.");
            return;
        };

        inspector::section(ui, "Settings", |ui| {
            Grid::new("texture_view_inspector_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Texture");
                    let mut texture_id = texture_view.texture_id();
                    let texture_before = texture_id;

                    inspector::storage_combo(
                        ui,
                        "texture",
                        &self.project.textures,
                        &mut texture_id,
                    );

                    ui.end_row();

                    if texture_id != texture_before {
                        texture_view.set_texture_id(texture_id);
                    }

                    ui.label("Format");
                    let mut current_format = texture_view.format();
                    let format_before = current_format;

                    ui.horizontal(|ui| {
                        const FORMAT_LIST: [Option<TextureViewFormat>; 3] = [
                            None,
                            Some(TextureViewFormat::Srgb),
                            Some(TextureViewFormat::Linear),
                        ];

                        inspector::value_combo(ui, "format", FORMAT_LIST, &mut current_format);
                    });

                    ui.end_row();

                    if current_format != format_before {
                        texture_view.set_format(current_format);
                    }

                    ui.label("Dimension");
                    let mut current_dimension = texture_view.dimension();
                    let dimension_before = current_dimension;

                    const DIMENSIONS: [Option<wgpu::TextureViewDimension>; 7] = [
                        None,
                        Some(wgpu::TextureViewDimension::D1),
                        Some(wgpu::TextureViewDimension::D2),
                        Some(wgpu::TextureViewDimension::D3),
                        Some(wgpu::TextureViewDimension::D2Array),
                        Some(wgpu::TextureViewDimension::Cube),
                        Some(wgpu::TextureViewDimension::CubeArray),
                    ];

                    inspector::value_combo(ui, "dimension", DIMENSIONS, &mut current_dimension);

                    ui.end_row();

                    if current_dimension != dimension_before {
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
                    ui.colored_label(ui.visuals().error_fg_color, err.to_string());
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

impl AsWidgetText for Option<TextureViewFormat> {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            Some(TextureViewFormat::Srgb) => "From Texture (Force sRGB)",
            Some(TextureViewFormat::Linear) => "From Texture (Force Linear)",
            None => "From Texture",
        };
        r.into()
    }
}

impl AsWidgetText for Option<wgpu::TextureViewDimension> {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            Some(wgpu::TextureViewDimension::D1) => "D1",
            Some(wgpu::TextureViewDimension::D2) => "D2",
            Some(wgpu::TextureViewDimension::D3) => "D3",
            Some(wgpu::TextureViewDimension::D2Array) => "D2Array",
            Some(wgpu::TextureViewDimension::Cube) => "Cube",
            Some(wgpu::TextureViewDimension::CubeArray) => "CubeArray",
            None => "From Texture",
        };
        r.into()
    }
}
