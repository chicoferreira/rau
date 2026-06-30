use serde::{Deserialize, Serialize};

use crate::{
    error::AppResult,
    project::{
        Creatable, DimensionId, ProjectResource,
        sync::{Revision, SyncOutcome, SyncResource, SyncTracker},
    },
    resource_getters, resource_setters,
    ui::size::Size2d,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dimension {
    label: String,
    size: DimensionSize,
    #[serde(skip)]
    runtime_revision: Revision,
    #[serde(skip)]
    project_revision: Revision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DimensionSize {
    Runtime(#[serde(skip)] Size2d),
    #[serde(untagged)]
    Persistent(Size2d),
}

impl Dimension {
    pub fn new(label: impl Into<String>, size: DimensionSize) -> Self {
        Self {
            label: label.into(),
            size,
            runtime_revision: Revision::default(),
            project_revision: Revision::default(),
        }
    }

    pub fn new_persistent(label: impl Into<String>, size: Size2d) -> Self {
        Self::new(label, DimensionSize::Persistent(size))
    }

    pub fn new_runtime(label: impl Into<String>) -> Self {
        Self::new(label, DimensionSize::Runtime(Size2d::default()))
    }

    resource_getters! {
        pub fn size() -> DimensionSize;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_label(label: String);
    }

    resource_setters! {
        increases: [runtime_revision, project_revision];
        pub fn set_size(size: DimensionSize);
    }

    pub fn get_actual_size(&self) -> Size2d {
        match &self.size {
            DimensionSize::Persistent(s) => *s,
            DimensionSize::Runtime(s) => *s,
        }
    }

    pub fn set_persistent(&mut self, persistent: bool) {
        let size = self.get_actual_size();
        self.set_size(match persistent {
            true => DimensionSize::Persistent(size),
            false => DimensionSize::Runtime(size),
        });
    }

    pub fn set_actual_size(&mut self, size: Size2d) {
        if self.get_actual_size() == size {
            return;
        }

        match &mut self.size {
            DimensionSize::Persistent(s) => {
                *s = size;
                self.project_revision.increase();
            }
            DimensionSize::Runtime(s) => *s = size,
        }
        self.runtime_revision.increase();
    }
}

impl Creatable for Dimension {
    fn create(label: String) -> Self {
        Self::new_persistent(label, Size2d::new(1920, 1080))
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
        _id: Self::Id,
        _ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
        _job: Self::Job,
    ) -> AppResult<SyncOutcome<Self::Runtime, Self::Job>> {
        Ok(SyncOutcome::Recreated(()))
    }

    fn needs_rebuild(&self, _: Self::Id, _: &Self::Context<'_>, _: &SyncTracker) -> bool {
        false
    }
}
