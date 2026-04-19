use crate::{
    error::AppResult,
    project::{
        DimensionId, ProjectResource,
        recreate::{Recreatable, RecreateTracker, Revision, SyncOutcome},
    },
    ui::Size2d,
};

pub struct Dimension {
    pub label: String,
    size: Size2d,
    revision: Revision,
}

impl Dimension {
    pub fn new(label: impl Into<String>, size: Size2d) -> Self {
        Self {
            label: label.into(),
            size,
            revision: Revision::default(),
        }
    }

    pub fn size(&self) -> Size2d {
        self.size
    }

    pub fn set_size(&mut self, size: Size2d) {
        if self.size != size {
            self.size = size;
            self.revision.increase();
        }
    }
}

impl ProjectResource for Dimension {
    type Id = DimensionId;

    fn label(&self) -> &str {
        &self.label
    }
}

impl Recreatable for Dimension {
    type Context<'a> = ();
    type Runtime = ();

    fn sync<'a>(
        &mut self,
        _ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        Ok(SyncOutcome::Recreated(()))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, _: &RecreateTracker) -> bool {
        false
    }
}
