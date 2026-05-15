use crate::{
    file::file_storage::FileStorage,
    project::{Project, ResourceId, paths::FilePath, sync::Revision},
};

type ProjectRevisionSnapshot = Vec<(ResourceId, Revision)>;

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

        let revisions = self.last_observed_snapshot.clone();

        match project.serialize() {
            Ok(bytes) => {
                file_storage.save_in_background(&FilePath::project_json(), bytes);
                self.saved_snapshot = revisions;
                self.save_deadline = None;
            }
            Err(error) => {
                log::error!("Failed to serialize project for save: {error}");
                self.save_deadline = Some(now + PROJECT_SAVE_DEBOUNCE);
            }
        }
    }
}
