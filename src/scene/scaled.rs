//! A scene made up of copies of another scene, for benchmarking purposes.

use std::collections::HashSet;

use crate::{
    error::AppResult,
    project::{Project, ResourceId, resource::dimension::DimensionSize},
    ui::size::Size2d,
};

const COPY_DIMENSION_SIDE: u32 = 64;

type SceneBuilder = fn(&mut Project) -> AppResult<()>;

pub fn build(project: &mut Project, scene: SceneBuilder, copies: usize) -> AppResult<()> {
    for copy in 1..copies {
        add_background_copy(project, scene, copy)?;
    }

    // This last one will be the presented scene, as it sets the
    // main viewport and the render passes in the presentation
    scene(project)?;

    let resource_count = project.project_revisions().count();
    log::info!("Generated a scaled project of {copies} copies ({resource_count} resources)",);

    Ok(())
}

fn add_background_copy(project: &mut Project, scene: SceneBuilder, copy: usize) -> AppResult<()> {
    let before: HashSet<ResourceId> = resource_ids(project).collect();
    scene(project)?;
    let registered: Vec<ResourceId> = resource_ids(project)
        .filter(|id| !before.contains(id))
        .collect();

    for id in registered {
        if let Some(label) = project.label(id) {
            project.set_label(id, format!("{label} #{copy}"));
        }

        match id {
            // Scale down the dimensions of the copies to avoid using too much memory for the textures
            ResourceId::Dimension(dimension_id) => {
                let small_size = Size2d::new(COPY_DIMENSION_SIDE, COPY_DIMENSION_SIDE);
                let small_dimension = DimensionSize::Persistent(small_size);

                let dimension = project.dimensions.get_mut(dimension_id)?;
                dimension.set_size(small_dimension);
            }
            // Prevent copied viewports from resizing their dimensions when opened
            ResourceId::Viewport(viewport_id) => {
                let viewport = project.viewports.get_mut(viewport_id)?;
                viewport.set_dimension_id(None);
            }
            _ => {}
        }
    }

    Ok(())
}

fn resource_ids(project: &Project) -> impl Iterator<Item = ResourceId> + '_ {
    project.project_revisions().map(|(id, _)| id)
}
