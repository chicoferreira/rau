fn main() {
    #[cfg(windows)]
    {
        compile_windows_resources();
    }

    // Generates `$OUT_DIR/built.rs` with build-time metadata (version, git
    // commit, build timestamp, target, profile, rustc version, ...) which is
    // included as the `built_info` module in `lib.rs` and surfaced in the
    // "Rau" menu.
    built::write_built_file().expect("Failed to acquire build-time information");
}

#[cfg(windows)]
fn compile_windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if !matches!(target_env.as_str(), "msvc" | "gnu") {
        return;
    }

    println!("cargo::rerun-if-changed=assets/rau-app-icon.ico");

    let mut resources = winresource::WindowsResource::new();
    resources.set_icon("assets/rau-app-icon.ico");
    resources
        .compile()
        .expect("Failed to compile Windows resources");
}
