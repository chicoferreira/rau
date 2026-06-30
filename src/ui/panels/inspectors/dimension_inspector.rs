use crate::{
    project::{DimensionId, resource::dimension::DimensionSize},
    ui::{
        components::{
            field,
            field_docs::field_doc,
            inspector::{self, AsRichText},
        },
        pane::StateSnapshot,
        size::Size2d,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum DimensionSizeKind {
    Persistent,
    Runtime,
}

impl AsRichText for DimensionSizeKind {
    fn as_rich_text(&self) -> egui::RichText {
        match self {
            DimensionSizeKind::Persistent => "Persistent",
            DimensionSizeKind::Runtime => "Runtime",
        }
        .into()
    }
}

impl AsRichText for Size2d {
    fn as_rich_text(&self) -> egui::RichText {
        format!("{} \u{00D7} {}", self.width(), self.height()).into()
    }
}

const SIZE_KINDS: [DimensionSizeKind; 2] =
    [DimensionSizeKind::Persistent, DimensionSizeKind::Runtime];

impl StateSnapshot<'_> {
    pub fn dimension_inspector_ui(&mut self, ui: &mut egui::Ui, dimension_id: DimensionId) {
        let Ok(dimension) = self.project.dimensions.get_mut(dimension_id) else {
            ui.label("Dimension couldn't be found.");
            return;
        };

        let actual_size = dimension.get_actual_size();
        let mut kind = match dimension.size() {
            DimensionSize::Persistent(_) => DimensionSizeKind::Persistent,
            DimensionSize::Runtime(_) => DimensionSizeKind::Runtime,
        };

        inspector::section_doc(
            ui,
            "Size",
            field_doc!(
                "A named **size** (width and height) that other resources can use.\n\n\
                Textures, cameras and viewports that use this Dimension all share this size, so \
                they update together when it changes. A viewport linked to this Dimension also \
                changes its size when you resize that viewport, in both modes. The modes only \
                differ in whether the size is saved.\n\n\
                - **Persistent**: the size is saved with the project. Set it below, or let a linked \
                viewport change it. Use this when the size should stay the same on every computer, \
                like a specific render size, a shadow map, or an image you export.\n\
                - **Runtime**: the size just follows a linked viewport and isn't saved. Use this \
                when the size should match the viewport on this computer, like the main viewport, \
                so each computer's window size isn't saved."
            ),
            |ui| {
                field::field_grid(ui, "dimension_inspector_grid", |ui| {
                    let size = (kind == DimensionSizeKind::Runtime).then_some(actual_size);
                    if field::row_doc(
                        ui,
                        "Mode",
                        field_doc!(
                            "Whether this Dimension's size is **Persistent** (saved with the project) or \
                            **Runtime** (only kept while the project is open). Both follow a linked \
                            viewport when you resize it; the only difference is whether the size is \
                            saved.\n\n\
                            Pick **Persistent** for sizes that belong to the project, like a specific \
                            render size, a shadow map, or an image you export. Pick **Runtime** for \
                            sizes that should just match the viewport on this computer."
                        ),
                        |ui| {
                            inspector::combo_with_weak(
                                ui,
                                "dimension_kind",
                                SIZE_KINDS,
                                &mut kind,
                                size,
                            )
                        },
                    ) {
                        dimension.set_persistent(kind == DimensionSizeKind::Persistent);
                    }

                    if kind == DimensionSizeKind::Persistent {
                        let mut width = actual_size.width();
                        let mut height = actual_size.height();

                        let mut changed = inspector::u32_drag_row_doc(
                            ui,
                            "Width",
                            field_doc!("The Dimension's **width**, in pixels."),
                            &mut width,
                            1_u32..=u32::MAX,
                        );
                        changed |= inspector::u32_drag_row_doc(
                            ui,
                            "Height",
                            field_doc!("The Dimension's **height**, in pixels."),
                            &mut height,
                            1_u32..=u32::MAX,
                        );

                        if changed {
                            dimension.set_actual_size(Size2d::new(width, height));
                        }
                    }
                });
            },
        );
    }
}
