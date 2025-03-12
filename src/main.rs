use anyhow::Context;

mod app;
mod file;
mod project;
mod renderer;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use pollster::FutureExt;
    let project = project::Project::from_file("project.toml")
        .block_on()
        .context("Failed to load project")?;
    let mut app = app::App::new(project);
    app.run().context("Failed to run app")?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(main)]
async fn main() -> anyhow::Result<()> {
    let project = project::Project::from_file("project.toml")
        .await
        .context("Failed to load project")?;
    let mut app = app::App::new(project);
    app.run().context("Failed to run app")?;

    Ok(())
}
