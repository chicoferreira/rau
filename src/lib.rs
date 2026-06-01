use winit::event_loop::EventLoop;

use crate::{app::App, error::AppResult, file::identifier::ProjectIdentifier, utils::winit_runner};

macro_rules! toasts_log_error {
    ($toasts:expr, $format:expr) => {
        let error = format!($format);
        log::error!("{error}");
        $toasts.error(error);
    };
}

mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
mod error;
mod featured_projects;
mod file;
mod main_menu;
mod project;
mod scene;
mod ui;
mod utils;
mod workspace;

#[derive(Default)]
pub enum StartupAction {
    #[default]
    MainMenu,
    OpenProject {
        project_id: ProjectIdentifier,
    },
    CreateEmptyProject {
        project_id: ProjectIdentifier,
    },
}

pub fn run(startup_action: StartupAction) -> AppResult<()> {
    let event_loop = EventLoop::<App>::with_user_event().build()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = winit_runner::WinitRunner::new(startup_action);
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;

        let app = winit_runner::WinitRunner::new(&event_loop, startup_action);
        event_loop.spawn_app(app)
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::UnwrapThrowExt;

    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).unwrap_throw();

    run(StartupAction::default()).unwrap_throw();

    Ok(())
}
