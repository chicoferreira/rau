fn main() {
    // Generates `$OUT_DIR/built.rs` with build-time metadata (version, git
    // commit, build timestamp, target, profile, rustc version, ...) which is
    // included as the `built_info` module in `lib.rs` and surfaced in the
    // "Rau" menu.
    built::write_built_file().expect("Failed to acquire build-time information");
}
