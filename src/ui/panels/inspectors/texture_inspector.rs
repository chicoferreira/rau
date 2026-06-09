use strum::IntoEnumIterator;
use wgpu::TextureUsages;

use crate::{
    project::{
        TextureId,
        paths::FilePath,
        resource::{dimension::Dimension, texture::TextureSource},
        storage::Storage,
    },
    ui::{
        components::{flags_selector::flags_selector, hint, inspector, selector::AsWidgetText},
        pane::StateSnapshot,
    },
    utils::texture_format::TextureFormat,
    workspace::StateEvent,
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

const TEXTURE_USAGES: &[(TextureUsages, &str)] = &[
    (TextureUsages::COPY_SRC, "Copy Source"),
    (TextureUsages::COPY_DST, "Copy Destination"),
    (TextureUsages::TEXTURE_BINDING, "Texture Binding"),
    (TextureUsages::STORAGE_BINDING, "Storage Binding"),
    (TextureUsages::RENDER_ATTACHMENT, "Render Attachment"),
    (TextureUsages::STORAGE_ATOMIC, "Storage Atomic"),
    (TextureUsages::TRANSIENT, "Transient"),
];

const SOURCE_KINDS: [TextureSourceKind; 3] = [
    TextureSourceKind::Image,
    TextureSourceKind::Dimension,
    TextureSourceKind::Manual,
];

impl AsWidgetText for TextureFormat {
    fn as_widget_text(&self) -> egui::WidgetText {
        self.label().into()
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

        inspector::field_grid(ui, "texture_inspector_grid", |ui| {
            inspector::combo_row(
                ui,
                "Format",
                "texture_format",
                TextureFormat::iter(),
                &mut format,
            );

            inspector::row(ui, "Usage", |ui| {
                flags_selector(ui, "texture_usage", &mut usage, TEXTURE_USAGES);
            });
        });

        ui_texture_source(
            ui,
            &mut source,
            &self.project.dimensions,
            self.file_storage.files(),
        );

        ui.add_space(6.0);

        ui.add(hint::hint(|ui| {
            ui.label("Create a Texture View to see the Texture contents.")
        }));

        if ui.button("Derive Texture View").clicked() {
            self.event_queue
                .add(StateEvent::DeriveTextureView(texture_id));
        }

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

fn ui_texture_source(
    ui: &mut egui::Ui,
    source: &mut TextureSource,
    dimensions: &Storage<Dimension>,
    files: Option<&[FilePath]>,
) {
    let current_kind = TextureSourceKind::from_source(source);
    let mut selected_kind = current_kind;

    inspector::field_grid(ui, "texture_source_grid", |ui| {
        if inspector::combo_row(
            ui,
            "Source",
            "texture_source_kind",
            SOURCE_KINDS,
            &mut selected_kind,
        ) {
            *source = match selected_kind {
                TextureSourceKind::Dimension => TextureSource::Dimension(None),
                TextureSourceKind::Manual => TextureSource::Manual {
                    size: wgpu::Extent3d {
                        width: 800,
                        height: 400,
                        depth_or_array_layers: 1,
                    },
                },
                TextureSourceKind::Image => TextureSource::Image(None),
            };
        }
    });

    ui.indent("source_options", |ui| {
        inspector::field_grid(ui, "texture_source_options_grid", |ui| match source {
            TextureSource::Dimension(dimension_id) => {
                inspector::storage_opt_combo_row(
                    ui,
                    "Dimension",
                    "texture_source_dimension",
                    dimensions,
                    dimension_id,
                );
            }
            TextureSource::Manual { size } => {
                inspector::u32_drag_row(ui, "Width", &mut size.width, 1_u32..=u32::MAX);
                inspector::u32_drag_row(ui, "Height", &mut size.height, 1_u32..=u32::MAX);
                inspector::u32_drag_row(
                    ui,
                    "Layers",
                    &mut size.depth_or_array_layers,
                    1_u32..=u32::MAX,
                );
            }
            TextureSource::Image(path) => {
                if let Some(files) = files {
                    inspector::file_opt_combo_row(
                        ui,
                        "Image",
                        "texture_source_image",
                        files,
                        path,
                        is_image_file,
                    );
                } else {
                    inspector::row(ui, "Image", |ui| {
                        ui.spinner();
                    });
                }
            }
        });
    });
}

fn is_image_file(path: &FilePath) -> bool {
    path.extension()
        .and_then(image::ImageFormat::from_extension)
        .is_some_and(|format| format.can_read() && format.reading_enabled())
}
