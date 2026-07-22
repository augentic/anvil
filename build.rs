//! Resolves the wasm32 engine component the shipped binary embeds.
//!
//! Emits `SPECIFY_ENGINE_WASM` for the `include_bytes!` in
//! `src/omnia.rs`. Resolution order: an explicit `SPECIFY_ENGINE_WASM`
//! environment override (the release pipeline), else the sibling
//! `target/wasm32-wasip2/<profile>/specify.wasm` build product (local
//! iteration — `cargo make wasm-build` orders the wasm32 build first),
//! else a generated empty placeholder so plain native `cargo build` /
//! clippy / nextest still compile. The placeholder fails at boot with
//! the component loader's error, never silently at run time.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=SPECIFY_ENGINE_WASM");

    let engine = std::env::var_os("SPECIFY_ENGINE_WASM").map_or_else(probe, |explicit| {
        let path = PathBuf::from(explicit);
        assert!(path.is_file(), "SPECIFY_ENGINE_WASM is set but {} is not a file", path.display());
        path
    });

    println!("cargo:rerun-if-changed={}", engine.display());
    println!("cargo:rustc-env=SPECIFY_ENGINE_WASM={}", engine.display());
}

/// The local wasm32 build product, else a generated empty placeholder.
///
/// The probe path is emitted as `rerun-if-changed` either way, so the
/// native binary re-embeds as soon as the engine guest is (re)built.
fn probe() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    let profile = std::env::var("PROFILE").expect("cargo env");
    let built = manifest_dir.join(format!("target/wasm32-wasip2/{profile}/specify.wasm"));
    if built.is_file() {
        return built;
    }
    // Track the missing product too: its appearance must trigger a rebuild.
    println!("cargo:rerun-if-changed={}", built.display());
    let placeholder = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env"))
        .join("engine-placeholder.wasm");
    std::fs::write(&placeholder, []).expect("write engine placeholder");
    placeholder
}
