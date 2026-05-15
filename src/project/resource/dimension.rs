use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    project::{
        Creatable, DimensionId, ProjectResource,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    ui::Size2d,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimension {
    label: String,
    size: Size2d,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

impl Dimension {
    pub fn new(label: impl Into<String>, size: Size2d) -> Self {
        Self {
            label: label.into(),
            size,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    pub fn size(&self) -> Size2d {
        self.size
    }

    pub fn set_label(&mut self, label: String) {
        if self.label != label {
            self.label = label;
            self.project_revision.increase();
        }
    }

    pub fn set_size(&mut self, size: Size2d) {
        if self.size != size {
            self.size = size;
            self.runtime_revision.increase();
            self.project_revision.increase();
        }
    }
}

impl Creatable for Dimension {
    fn create(label: String) -> Self {
        Self::new(label, Size2d::new(1920, 1080))
    }
}

impl ProjectResource for Dimension {
    type Id = DimensionId;

    fn label(&self) -> &str {
        &self.label
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}

impl SyncResource for Dimension {
    type Context<'a> = ();
    type Runtime = ();
    type Job = ();

    fn runtime_revision(&self) -> Revision {
        self.runtime_revision
    }

    fn sync<'a>(
        &self,
        _ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        _job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        Ok(SyncOutcome::Changed(()))
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }
}
