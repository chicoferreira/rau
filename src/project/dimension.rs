use crate::{
    error::AppResult,
    project::{
        DimensionId, ProjectResource,
        sync::{SyncResource, Revision, SyncOutcome, SyncTracker},
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

impl SyncResource for Dimension {
    type Context<'a> = ();
    type Runtime = ();

    fn sync<'a>(
        &mut self,
        _ctx: &mut Self::Context<'a>,
        _previous: Option<Self::Runtime>,
    ) -> AppResult<SyncOutcome<Self::Runtime>> {
        Ok(SyncOutcome::Changed(()))
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn needs_rebuild_from_others(&self, _: &SyncTracker) -> bool {
        false
    }
}
