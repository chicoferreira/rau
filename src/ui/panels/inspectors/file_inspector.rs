use crate::{
    file::file_storage::OpenFileState,
    project::paths::FilePath,
    ui::{
        components::{code_editor::code_editor, field, inspector},
        pane::StateSnapshot,
    },
};

const PREVIEW_SIZE_LIMIT: usize = 1024 * 1024;

impl StateSnapshot<'_> {
    pub fn file_inspector_ui(&mut self, ui: &mut egui::Ui, file_path: &FilePath) {
        let project_name = self.file_storage.project_source().project_name();
        let force_id = egui::Id::new(("force_file_preview", project_name, file_path));

        match self.file_storage.open_file(file_path) {
            OpenFileState::Loading { .. } => {
                field::centered(ui, field::spinner);
            }
            OpenFileState::Loaded { text, saved }
            | OpenFileState::Reloading { text, saved, .. } => {
                let forced = ui
                    .ctx()
                    .data(|data| data.get_temp(force_id))
                    .unwrap_or(false);

                if text.len() > PREVIEW_SIZE_LIMIT && !forced {
                    too_large_preview(ui, text.len(), force_id);
                    return;
                }

                let extension = file_path.extension().unwrap_or("");
                if code_editor(ui, text, extension).changed() && text != &saved.text {
                    let contents = text.clone();
                    self.file_storage.save_open_file(file_path, contents);
                }
            }
            OpenFileState::Errored { error } => {
                inspector::centered_error(ui, format!("Couldn't open file:\n{error}"));
            }
        };
    }
}

fn too_large_preview(ui: &mut egui::Ui, size: usize, force_id: egui::Id) {
    inspector::centered_block(ui, |ui| {
        ui.label(format!(
            "This file is too large to preview ({}).",
            format_size(size)
        ));
        field::weak_label(ui, "Previewing it may make the editor unresponsive.");
        ui.add_space(8.0);
        if ui.button("Preview anyway").clicked() {
            ui.ctx().data_mut(|data| data.insert_temp(force_id, true));
        }
    });
}

fn format_size(bytes: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = KIB * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}
