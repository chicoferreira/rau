fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    rau::cli::main();
}
