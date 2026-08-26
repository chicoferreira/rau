fn main() {
    #[cfg(windows)]
    {
        compile_windows_resources();
    }

    if std::path::Path::new(".git/logs/HEAD").exists() {
        println!("cargo::rerun-if-changed=.git/logs/HEAD");
    }

    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=Cargo.toml");
    println!("cargo::rerun-if-changed=Cargo.lock");

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
