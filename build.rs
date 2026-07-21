//! Root-crate build script: optional engine-component embedding.
//!
//! When `SPECIFY_ENGINE_WASM` names the built wasm32 engine cdylib at
//! build time (the release pipeline builds the guest first), the path
//! is re-exported to the compiler along with the `engine_embedded`
//! cfg, and `src/omnia.rs` embeds the bytes via `include_bytes!`.
//! Builds without the variable compile exactly as before: the
//! launcher falls back to registry hydration for the engine entry.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(engine_embedded)");
    println!("cargo::rerun-if-env-changed=SPECIFY_ENGINE_WASM");
    let Some(path) = std::env::var_os("SPECIFY_ENGINE_WASM").filter(|path| !path.is_empty()) else {
        return;
    };
    // `include_bytes!` resolves relative to the including source file,
    // so the re-exported path must be absolute; canonicalizing also
    // fails the build early when the component does not exist.
    let component = std::fs::canonicalize(&path).unwrap_or_else(|err| {
        panic!(
            "SPECIFY_ENGINE_WASM does not name a readable engine component ({}): {err}",
            std::path::Path::new(&path).display(),
        )
    });
    println!("cargo::rerun-if-changed={}", component.display());
    println!("cargo::rustc-env=SPECIFY_ENGINE_WASM={}", component.display());
    println!("cargo::rustc-cfg=engine_embedded");
}
