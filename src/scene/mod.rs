use std::path::Path;

use crate::{
    error::AppResult,
    file::{
        absolute::AbsolutePathBuf,
        file_storage::FileStorage,
        file_system::{AppFileSystem, ProjectFileSystem, ProjectFileSystemTrait},
        identifier::{ProjectIdentifier, ProjectSource},
    },
    project::{Project, paths::FilePath},
    scene::GenerateTemplate::*,
};

pub mod area_lights;
pub mod depth_testing;
pub mod fur_shell;
pub mod game_of_life;
pub mod grass_field;
pub mod hdr_skybox;
pub mod model;
pub mod parallax_mapping;
pub mod ray_tracing;
pub mod scaled;
pub mod shadow_mapping;
pub mod sky_shader;
pub mod ssao;

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum GenerateTemplate {
    HdrSkybox,
    FurShell,
    Model,
    GameOfLife,
    ParallaxMapping,
    GrassField,
    DepthTesting,
    ShadowMapping,
    SkyShader,
    Ssao,
    AreaLights,
    RayTracing,
    #[value(name = "shadow-mapping-10-copies")]
    ShadowMapping10Copies,
    #[value(name = "shadow-mapping-100-copies")]
    ShadowMapping100Copies,
}

pub fn generate_project(template: GenerateTemplate, target_folder: &Path) -> AppResult<()> {
    pollster::block_on(generate_project_async(template, target_folder))
}

async fn generate_project_async(template: GenerateTemplate, target_folder: &Path) -> AppResult<()> {
    let device = request_device().await?;
    let (file_system, file_storage) = request_file_system(target_folder).await?;

    let mut project = Project::default();

    match template {
        HdrSkybox => hdr_skybox::build(&mut project, &device, &file_storage).await?,
        Model => model::build(&mut project, &device, &file_storage).await?,
        GameOfLife => game_of_life::build(&mut project)?,
        FurShell => fur_shell::build(&mut project)?,
        ParallaxMapping => parallax_mapping::build(&mut project)?,
        GrassField => grass_field::build(&mut project)?,
        DepthTesting => depth_testing::build(&mut project)?,
        ShadowMapping => shadow_mapping::build(&mut project)?,
        SkyShader => sky_shader::build(&mut project)?,
        Ssao => ssao::build(&mut project)?,
        AreaLights => area_lights::build(&mut project)?,
        RayTracing => ray_tracing::build(&mut project)?,
        ShadowMapping10Copies => scaled::build(&mut project, shadow_mapping::build, 10)?,
        ShadowMapping100Copies => scaled::build(&mut project, shadow_mapping::build, 100)?,
    };

    file_system
        .write(&FilePath::project_json(), project.serialize()?)
        .await?;

    log::info!("Generated project.json into {}", target_folder.display());
    Ok(())
}

async fn request_file_system(target_folder: &Path) -> AppResult<(ProjectFileSystem, FileStorage)> {
    let app_file_system = AppFileSystem::open().await?;
    let project_id = ProjectIdentifier::new(
        "generated-project",
        AbsolutePathBuf::new(target_folder.to_path_buf())?,
    );
    let source = ProjectSource::Persistent(project_id);
    let (file_system, file_watcher) = app_file_system.mount_project(source.clone()).await?;
    let file_storage = FileStorage::new(source, file_system.clone(), file_watcher);
    Ok((file_system, file_storage))
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
