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
        components::{
            field_docs::field_doc,
            flags_selector::flags_selector,
            hint,
            inspector::{self, AsWidgetText},
            resource_icons,
        },
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

        inspector::section(ui, "Settings", |ui| {
            inspector::field_grid(ui, "texture_inspector_grid", |ui| {
                inspector::combo_row_doc(
                    ui,
                    "Format",
                    field_doc!(
                        "How each **texel** is laid out in memory.\n\n\
                        The format fixes three things:\n\
                        - **Channels**: which components are stored (`R`, `RG`, `RGBA`, depth, ...).\n\
                        - **Bit depth & type** of each channel: e.g. `8` bits as `Unorm` \
                        (0 to 1 normalized), `Uint`, or `32`-bit `Float`.\n\
                        - **Color space**: `Srgb` formats are decoded to linear on read and \
                        encoded back on write; non-`Srgb` formats are treated as raw linear values.\n\n\
                        Example: `Rgba8UnormSrgb` is 4 channels at 8 bits each = **4 bytes per texel**, \
                        sampled in sRGB.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#enumdef-gputextureformat)"
                    ),
                    "texture_format",
                    TextureFormat::iter(),
                    &mut format,
                );

                inspector::row_doc(
                    ui,
                    "Usage",
                    field_doc!(
                        "Which operations this texture must support. The GPU validates every \
                        use against these flags, so enable only what is needed.\n\n\
                        - **Texture Binding**: sampled in a shader.\n\
                        - **Storage Binding**: read/written as a storage texture.\n\
                        - **Render Attachment**: drawn into by a render pass.\n\
                        - **Copy Source / Destination**: used in copy operations.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#namespacedef-gputextureusage)"
                    ),
                    |ui| {
                        flags_selector(ui, "texture_usage", &mut usage, TEXTURE_USAGES);
                    },
                );
            });

            ui_texture_source(
                ui,
                &mut source,
                &self.project.dimensions,
                self.file_storage.files(),
            );
        });

        inspector::section(ui, "Texture View", |ui| {
            ui.add(hint::hint(|ui| {
                ui.label("Create a Texture View to see the Texture contents.")
            }));

            if ui
                .button(resource_icons::derive_text(ui, "Derive Texture View"))
                .clicked()
            {
                self.event_queue
                    .add(StateEvent::DeriveTextureView(texture_id));
            }
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

fn ui_texture_source(
    ui: &mut egui::Ui,
    source: &mut TextureSource,
    dimensions: &Storage<Dimension>,
    files: Option<&[FilePath]>,
) {
    let current_kind = TextureSourceKind::from_source(source);
    let mut selected_kind = current_kind;

    inspector::field_grid(ui, "texture_source_grid", |ui| {
        if inspector::combo_row_doc(
            ui,
            "Source",
            field_doc!(
                "Where this texture's **size** and initial **contents** come from.\n\n\
                Only the **Image** source uploads pixel data; the other two just allocate an \
                **empty** texture that a render pass, compute pass, or copy is expected to fill.\n\n\
                - **Image**: decode an image file and upload it as the texture's contents. The \
                size is taken from the image.\n\
                - **Dimension**: allocate an empty texture that tracks the size of a Dimension \
                resource, resizing automatically with it. Typically used as a \
                render target (e.g. a viewport).\n\
                - **Manual**: allocate an empty texture at a fixed width, height and layer count \
                you enter below."
            ),
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
                inspector::row_doc(
                    ui,
                    "Dimension",
                    field_doc!(
                        "The Dimension resource whose size this texture mirrors. Resizing the \
                        dimension (e.g. by resizing its viewport) recreates the texture to match."
                    ),
                    |ui| {
                        inspector::storage_combo(
                            ui,
                            "texture_source_dimension",
                            dimensions,
                            dimension_id,
                        );
                    },
                );
            }
            TextureSource::Manual { size } => {
                inspector::u32_drag_row_doc(
                    ui,
                    "Width",
                    field_doc!(
                        "Texture **width**, in texels.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuextent3ddict-width)"
                    ),
                    &mut size.width,
                    1_u32..=u32::MAX,
                );
                inspector::u32_drag_row_doc(
                    ui,
                    "Height",
                    field_doc!(
                        "Texture **height**, in texels.\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuextent3ddict-height)"
                    ),
                    &mut size.height,
                    1_u32..=u32::MAX,
                );
                inspector::u32_drag_row_doc(
                    ui,
                    "Layers",
                    field_doc!(
                        "Number of **array layers** (or depth slices for 3D textures).\n\n\
                        [WebGPU spec](https://www.w3.org/TR/webgpu/#dom-gpuextent3ddict-depthorarraylayers)"
                    ),
                    &mut size.depth_or_array_layers,
                    1_u32..=u32::MAX,
                );
            }
            TextureSource::Image(path) => {
                if let Some(files) = files {
                    inspector::row_doc(
                        ui,
                        "Image",
                        field_doc!(
                            "The image file decoded into this texture's contents. \
                            Supported formats include PNG, JPEG and HDR."
                        ),
                        |ui| {
                            inspector::file_combo(
                                ui,
                                "texture_source_image",
                                files,
                                path,
                                is_image_file,
                            );
                        },
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
