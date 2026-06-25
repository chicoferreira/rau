use std::path::Path;

use crate::{
    error::AppResult,
    file::{
        absolute::AbsolutePathBuf,
        file_storage::FileStorage,
        file_system::{AppFileSystem, ProjectFileSystemTrait},
        identifier::{ProjectIdentifier, ProjectSource},
    },
    project::paths::FilePath,
    ui::size::Size2d,
};

pub mod full_example;
pub mod game_of_life;
pub mod grass_field;
pub mod heightmap;
pub mod model;
pub mod parallax_mapping;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum GenerateTemplate {
    FullExample,
    Model,
    GameOfLife,
    Heightmap,
    ParallaxMapping,
    GrassField,
}

pub fn generate_project(template: GenerateTemplate, target_folder: &Path) -> AppResult<()> {
    pollster::block_on(generate_project_async(template, target_folder))
}

async fn generate_project_async(template: GenerateTemplate, target_folder: &Path) -> AppResult<()> {
    let device = request_device().await?;
    let app_file_system = AppFileSystem::open().await?;

    let project_id = ProjectIdentifier::new(
        "generated-project",
        AbsolutePathBuf::new(target_folder.to_path_buf())?,
    );
    let source = ProjectSource::Persistent(project_id);
    let (file_system, file_watcher) = app_file_system.mount_project(source.clone()).await?;
    let file_storage = FileStorage::new(source, file_system.clone(), file_watcher);

    let size = Size2d::new(1080, 1080);
    let project = match template {
        GenerateTemplate::FullExample => {
            full_example::create_scene(&device, size, &file_storage).await?
        }
        GenerateTemplate::Model => model::create_scene(&device, size, &file_storage).await?,
        GenerateTemplate::GameOfLife => {
            game_of_life::create_scene(&device, size, &file_storage).await?
        }
        GenerateTemplate::Heightmap => {
            heightmap::create_scene(&device, size, &file_storage).await?
        }
        GenerateTemplate::ParallaxMapping => {
            parallax_mapping::create_scene(&device, size, &file_storage).await?
        }
        GenerateTemplate::GrassField => {
            grass_field::create_scene(&device, size, &file_storage).await?
        }
    };

    file_system
        .write(&FilePath::project_json(), project.serialize()?)
        .await?;

    log::info!("Generated project.json into {}", target_folder.display());
    Ok(())
}

async fn request_device() -> AppResult<wgpu::Device> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;
    let (device, _queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?;
    Ok(device)
}
