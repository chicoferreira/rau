use std::hash::Hash;

use egui::{RichText, Ui};

use crate::{
    project::{
        resource::dimension::{Axis, Dimension, DimensionRef},
        storage::Storage,
    },
    ui::components::{
        field,
        inspector::{self, AsRichText},
    },
};

const AXES: [Axis; 2] = [Axis::Width, Axis::Height];

impl AsRichText for Axis {
    fn as_rich_text(&self) -> RichText {
        match self {
            Axis::Width => "Width",
            Axis::Height => "Height",
        }
        .into()
    }
}

pub fn dimension_ref_edit(
    ui: &mut Ui,
    id_salt: impl Hash,
    dimensions: &Storage<Dimension>,
    value: &mut DimensionRef,
) -> bool {
    ui.horizontal(|ui| {
        let mut changed =
            inspector::storage_combo(ui, (&id_salt, "dimension"), dimensions, &mut value.id);
        changed |= inspector::value_combo(ui, (&id_salt, "axis"), AXES, &mut value.axis);

        if let Ok(resolved) = value.resolve(dimensions) {
            field::weak_label(ui, format!("= {resolved}"));
        }

        changed
    })
    .inner
}
