use egui::{CollapsingHeader, Grid, load::SizedTexture};

use crate::{
    project::{TextureViewId, texture_view::TextureViewFormat},
    ui::{
        components::selector::{selectable_value, selectable_value_storage},
        pane::StateSnapshot,
    },
};

impl StateSnapshot<'_> {
    pub fn texture_view_inspector_ui(&mut self, ui: &mut egui::Ui, texture_view_id: TextureViewId) {
        let Some(texture_view) = self.project.texture_views.get_mut(texture_view_id) else {
            ui.label("Texture View couldn't be found.");
            return;
        };

        Grid::new("texture_view_inspector_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Texture");
                let mut texture_id = Some(texture_view.texture_id());
                let texture_before = texture_id;
                selectable_value_storage(
                    ui,
                    "texture",
                    &mut texture_id,
                    |_, texture| texture.label.to_string(),
                    &mut self.project.textures,
                );
                ui.end_row();

                if texture_id != texture_before {
                    texture_view.set_texture_id(texture_id.unwrap());
                }

                ui.label("Format");
                let format_name = |format: Option<TextureViewFormat>| match format {
                    Some(TextureViewFormat::Srgb) => "From Texture (Force sRGB)",
                    Some(TextureViewFormat::Linear) => "From Texture (Force Linear)",
                    None => "From Texture",
                };
                let mut current_format = texture_view.format();
                let format_before = current_format;

                ui.horizontal(|ui| {
                    selectable_value(
                        ui,
                        "format",
                        &mut current_format,
                        format_name,
                        [
                            None,
                            Some(TextureViewFormat::Srgb),
                            Some(TextureViewFormat::Linear),
                        ],
                    );
                });

                ui.end_row();

                if current_format != format_before {
                    texture_view.set_format(current_format);
                }

                ui.label("Dimension");
                let mut current_dimension = texture_view.dimension();
                let dimension_before = current_dimension;
                let format_dimension = |d| match d {
                    Some(wgpu::TextureViewDimension::D1) => "1D",
                    Some(wgpu::TextureViewDimension::D2) => "2D",
                    Some(wgpu::TextureViewDimension::D3) => "3D",
                    Some(wgpu::TextureViewDimension::D2Array) => "2D Array",
                    Some(wgpu::TextureViewDimension::Cube) => "Cube",
                    Some(wgpu::TextureViewDimension::CubeArray) => "Cube Array",
                    None => "From Texture",
                };

                selectable_value(
                    ui,
                    "dimension",
                    &mut current_dimension,
                    format_dimension,
                    [
                        None,
                        Some(wgpu::TextureViewDimension::D1),
                        Some(wgpu::TextureViewDimension::D2),
                        Some(wgpu::TextureViewDimension::D3),
                        Some(wgpu::TextureViewDimension::D2Array),
                        Some(wgpu::TextureViewDimension::Cube),
                        Some(wgpu::TextureViewDimension::CubeArray),
                    ],
                );
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
