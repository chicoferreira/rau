use egui::{CollapsingHeader, Grid, load::SizedTexture};

use crate::{
    project::{TextureViewId, texture_view::TextureViewFormat},
    ui::{
        components::selector::{AsWidgetText, ComboBoxExt},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn texture_view_inspector_ui(&mut self, ui: &mut egui::Ui, texture_view_id: TextureViewId) {
        let Ok(texture_view) = self.project.texture_views.get_mut(texture_view_id) else {
            ui.label("Texture View couldn't be found.");
            return;
        };

        Grid::new("texture_view_inspector_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Texture");
                let mut texture_id = texture_view.texture_id();
                let texture_before = texture_id;

                egui::ComboBox::from_id_salt("texture")
                    .selected_text_storage_opt(&self.project.textures, texture_id)
                    .show_ui_storage_opt(ui, &self.project.textures, &mut texture_id);

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

                    egui::ComboBox::from_id_salt("format")
                        .selected_text(current_format.as_widget_text())
                        .show_ui_list(ui, FORMAT_LIST, &mut current_format);
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

                egui::ComboBox::from_id_salt("dimension")
                    .selected_text(current_dimension.as_widget_text())
                    .show_ui_list(ui, DIMENSIONS, &mut current_dimension);

                ui.end_row();

                if current_dimension != dimension_before {
                    texture_view.set_dimension(current_dimension);
                }
            });

        CollapsingHeader::new("Preview")
            .default_open(true)
            .show(ui, |ui| {
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
