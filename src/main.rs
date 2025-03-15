use anyhow::Context;

mod app;
mod file;
mod project;
mod renderer;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    env_logger::init();
    use pollster::FutureExt;
    let project = project::Project::from_file("assets/project.toml")
        .block_on()
        .context("Failed to load project")?;
    let mut app = app::App::new(project);
    app.run().context("Failed to run app")?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(main)]
async fn main() -> anyhow::Result<()> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init().expect("could not initialize logger");

    let project = project::Project::from_file("assets/project.toml")
        .await
        .context("Failed to load project")?;
    let mut app = app::App::new(project);
    app.run().context("Failed to run app")?;

    Ok(())
}
