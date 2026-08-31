#[cfg(all(not(target_arch = "wasm32"), feature = "mimalloc"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    rau::startup::cli::main();
}
