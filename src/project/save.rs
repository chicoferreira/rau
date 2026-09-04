use crate::{
    file::file_storage::FileStorage,
    project::{Project, ProjectRevisionSnapshot, paths::FilePath},
};

const PROJECT_SAVE_DEBOUNCE: instant::Duration = instant::Duration::from_millis(500);

pub struct ProjectSaveState {
    last_observed_snapshot: ProjectRevisionSnapshot,
    saved_snapshot: ProjectRevisionSnapshot,
    save_deadline: Option<instant::Instant>,
}

impl ProjectSaveState {
    pub fn new(project: &Project) -> Self {
        let revisions: ProjectRevisionSnapshot = project.project_revisions().collect();

        Self {
            last_observed_snapshot: revisions.clone(),
            saved_snapshot: revisions,
            save_deadline: None,
        }
    }

    pub fn tick(&mut self, project: &Project, file_storage: &mut FileStorage) {
        puffin::profile_function!();

        let now = instant::Instant::now();
        let revisions = project.project_revisions().collect();

        if revisions != self.last_observed_snapshot {
            self.last_observed_snapshot = revisions;
            self.save_deadline = if self.last_observed_snapshot != self.saved_snapshot {
                Some(now + PROJECT_SAVE_DEBOUNCE)
            } else {
                None
            };
            return;
        }

        let Some(save_deadline) = self.save_deadline else {
            return;
        };

        if now < save_deadline {
            return;
        }

        self.save(project, file_storage);
    }

    /// Immediately saves the current project, bypassing the autosave debounce.
    pub fn save(&mut self, project: &Project, file_storage: &mut FileStorage) {
        let revisions: ProjectRevisionSnapshot = project.project_revisions().collect();
        let serialize_started = instant::Instant::now();

        match project.serialize() {
            Ok(bytes) => {
                log::info!(
                    "Serialized project in {:.1?} ({} bytes), saving...",
                    serialize_started.elapsed(),
                    bytes.len()
                );
                file_storage.save_in_background(&FilePath::project_json(), bytes);
                self.last_observed_snapshot = revisions.clone();
                self.saved_snapshot = revisions;
                self.save_deadline = None;
            }
            Err(error) => {
                log::error!("Failed to serialize project for save: {error}");
                self.save_deadline = Some(instant::Instant::now() + PROJECT_SAVE_DEBOUNCE);
            }
        }
    }
}
