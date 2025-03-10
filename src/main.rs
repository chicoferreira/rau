use anyhow::Context;

mod app;
mod project;
mod renderer;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let project = project::Project::from_file("project.toml").context("Failed to load project")?;
    let mut app = app::App::new(project);
    app.run().context("Failed to run app")?;

    Ok(())
}
