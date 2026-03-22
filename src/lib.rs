use winit::event_loop::EventLoop;

mod app;
mod error;
mod key;
mod model;
mod project;
mod render;
mod resources;
mod scene;
mod state;
mod ui;

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::builder()
            .parse_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();

        let event_loop = EventLoop::with_user_event().build()?;
        let mut app = app::App::new();

        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;

        console_log::init_with_level(log::Level::Info)?;

        let event_loop = EventLoop::with_user_event().build()?;
        let app = app::App::new(&event_loop);

        event_loop.spawn_app(app)
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::UnwrapThrowExt;

    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
