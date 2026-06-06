fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    rau::startup::cli::main();
}
