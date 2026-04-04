use egui::{ComboBox, Grid, PopupCloseBehavior, Ui, Widget};
use wgpu::TextureUsages;

use crate::{
    project::{DimensionId, TextureId, texture::TextureSource},
    ui::{
        components::selector::{AsWidgetText, ComboBoxExt},
        pane::StateSnapshot,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum TextureSourceKind {
    Dimension,
    Manual,
    Image,
}

impl TextureSourceKind {
    fn from_source(source: &TextureSource) -> Self {
        match source {
            TextureSource::Dimension(_) => Self::Dimension,
            TextureSource::Manual { .. } => Self::Manual,
            TextureSource::Image(_) => Self::Image,
        }
    }
}

impl AsWidgetText for TextureSourceKind {
    fn as_widget_text(&self) -> egui::WidgetText {
        let r = match self {
            Self::Dimension => "Dimension",
            Self::Manual => "Manual",
            Self::Image => "Image",
        };
        r.into()
    }
}

const TEXTURE_FORMATS: [wgpu::TextureFormat; 7] = [
    wgpu::TextureFormat::Rgba8UnormSrgb,
    wgpu::TextureFormat::Rgba8Unorm,
    wgpu::TextureFormat::Rgba16Float,
    wgpu::TextureFormat::Rgba32Float,
    wgpu::TextureFormat::Depth32Float,
    wgpu::TextureFormat::Depth24Plus,
    wgpu::TextureFormat::Depth24PlusStencil8,
];

impl AsWidgetText for wgpu::TextureFormat {
    fn as_widget_text(&self) -> egui::WidgetText {
        match self {
            wgpu::TextureFormat::Rgba8UnormSrgb => "RGBA8 Unorm sRGB".into(),
            wgpu::TextureFormat::Rgba8Unorm => "RGBA8 Unorm Linear".into(),
            wgpu::TextureFormat::Rgba16Float => "RGBA16 Float".into(),
            wgpu::TextureFormat::Rgba32Float => "RGBA32 Float".into(),
            wgpu::TextureFormat::Depth32Float => "Depth32 Float".into(),
            wgpu::TextureFormat::Depth24Plus => "Depth24 Plus".into(),
            wgpu::TextureFormat::Depth24PlusStencil8 => "Depth24 Plus Stencil8".into(),
            r => format!("{:?}", r).into(),
        }
    }
}

impl StateSnapshot<'_> {
    pub fn texture_inspector_ui(&mut self, ui: &mut egui::Ui, texture_id: TextureId) {
        let Ok(texture) = self.project.textures.get_mut(texture_id) else {
            ui.label("Texture couldn't be found.");
            return;
        };

        let mut format = texture.format();
        let format_before = format;

        let mut usage = texture.usage();
        let usage_before = usage;

        let source_before = texture.source().clone();
        let mut source = source_before.clone();

        Grid::new("texture_inspector_grid")
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                ui.label("Format");
                egui::ComboBox::from_id_salt("texture_format")
                    .selected_text(format.as_widget_text())
                    .show_ui_list(ui, TEXTURE_FORMATS, &mut format);

                ui.end_row();

                ui.label("Usage");
                texture_usages_widget(ui, &mut usage);
                ui.end_row();

                ui.label("Source");
                ui_texture_source(ui, &mut source, &self.project.dimensions);
                ui.end_row();
            });

        if format != format_before {
            texture.set_format(format);
        }

        if usage != usage_before {
            texture.set_usage(usage);
        }

        if source != source_before {
            texture.set_source(source);
        }
    }
}

fn texture_usages_widget(ui: &mut Ui, usages: &mut TextureUsages) -> bool {
    const FLAGS: &[(TextureUsages, &str)] = &[
        (TextureUsages::COPY_SRC, "COPY_SRC"),
        (TextureUsages::COPY_DST, "COPY_DST"),
        (TextureUsages::TEXTURE_BINDING, "TEXTURE_BINDING"),
        (TextureUsages::STORAGE_BINDING, "STORAGE_BINDING"),
        (TextureUsages::RENDER_ATTACHMENT, "RENDER_ATTACHMENT"),
        (TextureUsages::STORAGE_ATOMIC, "STORAGE_ATOMIC"),
        (TextureUsages::TRANSIENT, "TRANSIENT"),
    ];

    let before = *usages;

    let summary = if usages.is_empty() {
        "None".to_owned()
    } else {
        FLAGS
            .iter()
            .filter_map(|(flag, name)| usages.contains(*flag).then_some(*name))
            .collect::<Vec<_>>()
            .join(" | ")
    };

    ComboBox::from_id_salt("texture_usage")
        .selected_text(summary)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show_ui(ui, |ui| {
            for &(flag, name) in FLAGS {
                let mut enabled = usages.contains(flag);
                if ui.checkbox(&mut enabled, name).changed() {
                    usages.set(flag, enabled);
                }
            }
        });

    *usages != before
}

fn ui_texture_source(
    ui: &mut egui::Ui,
    source: &mut TextureSource,
    dimensions: &crate::project::storage::Storage<
        DimensionId,
        crate::project::dimension::Dimension,
    >,
) {
    let current_kind = TextureSourceKind::from_source(source);
    let mut selected_kind = current_kind;

    const SOURCE_KINDS: [TextureSourceKind; 3] = [
        TextureSourceKind::Image,
        TextureSourceKind::Dimension,
        TextureSourceKind::Manual,
    ];

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("texture_source_kind")
            .selected_text(selected_kind.as_widget_text())
            .show_ui_list(ui, SOURCE_KINDS, &mut selected_kind);

        if selected_kind != current_kind {
            *source = match selected_kind {
                // TODO: Make dimension optional
                TextureSourceKind::Dimension => first_dimension_id(dimensions)
                    .map(TextureSource::Dimension)
                    .unwrap_or(TextureSource::Manual {
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                    }),
                TextureSourceKind::Manual => TextureSource::Manual {
                    size: wgpu::Extent3d {
                        width: 800,
                        height: 400,
                        depth_or_array_layers: 1,
                    },
                },
                TextureSourceKind::Image => source.clone(),
            };
        }

        match source {
            TextureSource::Dimension(dimension_id) => {
                let mut selected_dimension = Some(*dimension_id);
                egui::ComboBox::from_id_salt("texture_source_dimension")
                    .selected_text_storage_opt(dimensions, selected_dimension)
                    .show_ui_storage_opt(ui, dimensions, &mut selected_dimension);

                if let Some(new_dimension_id) = selected_dimension {
                    *dimension_id = new_dimension_id;
                }
            }
            TextureSource::Manual { size } => {
                ui.horizontal(|ui| {
                    ui.label("W");
                    egui::DragValue::new(&mut size.width)
                        .range(1_u32..=u32::MAX)
                        .speed(1)
                        .ui(ui);
                    ui.label("H");
                    egui::DragValue::new(&mut size.height)
                        .range(1_u32..=u32::MAX)
                        .speed(1)
                        .ui(ui);
                    ui.label("Layers");
                    egui::DragValue::new(&mut size.depth_or_array_layers)
                        .range(1_u32..=u32::MAX)
                        .speed(1)
                        .ui(ui);
                });
            }
            TextureSource::Image(image) => {
                let width = image.width();
                let height = image.height();
                ui.label(format!("{width} x {height}"));
            }
        }
    });
    ui.label("Create a Texture View to see the Texture contents.");
}

fn first_dimension_id(
    dimensions: &crate::project::storage::Storage<
        DimensionId,
        crate::project::dimension::Dimension,
    >,
) -> Option<DimensionId> {
    dimensions.list().next().map(|(id, _)| id)
}
