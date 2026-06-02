use serde::{Deserialize, Serialize};

use crate::{
    project::{PresentationId, ProjectResource, RenderPassId, ViewportId, sync::Revision},
    resource_getters, resource_setters,
};

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Presentation {
    render_passes: Vec<RenderPassId>,
    main_viewport: Option<ViewportId>,
    #[serde(skip)]
    project_revision: Revision,
}

impl Presentation {
    resource_getters! {
        pub fn render_passes() -> &[RenderPassId];
        pub fn main_viewport() -> Option<ViewportId>;
    }

    resource_setters! {
        increases: [project_revision];
        pub fn set_render_passes(render_passes: Vec<RenderPassId>);
        pub fn set_main_viewport(main_viewport: Option<ViewportId>);
    }
}

impl ProjectResource for Presentation {
    type Id = PresentationId;

    fn label(&self) -> &str {
        "Presentation"
    }

    fn project_revision(&self) -> Revision {
        self.project_revision
    }
}
