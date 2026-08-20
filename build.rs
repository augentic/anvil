//! Builds the wasm32 engine with a child cargo build and embeds it at
//! `$OUT_DIR/emery.cwasm` (AOT-serialized in release, raw in debug).

use std::path::PathBuf;
use std::process::Command;

// The engine guest's compilation target.
const WASM_TARGET: &str = "wasm32-wasip2";

// Install instruction surfaced whenever the child build cannot run.
const TARGET_HINT: &str = "the wasm32 engine could not be built; install the target with `rustup \
                           target add wasm32-wasip2` and retry";

fn main() {
    // The wasm32 build runs this script too (the engine guest cdylib
    // embeds nothing); returning here is the recursion guard for the
    // child build below.
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let engine = build_engine();

    let len = std::fs::metadata(&engine).map(|meta| meta.len()).unwrap_or_default();
    assert!(len > 0, "engine component at {} is empty; refusing to embed it", engine.display());

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env")).join("emery.cwasm");
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        precompile(&engine, &out);
    } else {
        // Debug embeds the raw component (JIT at startup): the AOT
        // pass costs Cranelift time on every engine rebuild, which is
        // pure overhead in the edit loop and CI.
        std::fs::copy(&engine, &out).unwrap_or_else(|err| {
            panic!("copying {} to {}: {err}", engine.display(), out.display())
        });
    }
}

// Ahead-of-time compile the raw engine component into the serialized
// wasmtime artifact `src/main.rs` embeds. The env-driven `RuntimeOptions`
// must match the running binary (cargo's `TARGET` pins the triple).
fn precompile(raw: &std::path::Path, out: &std::path::Path) {
    let options = omnia::RuntimeOptions::load_env().expect("runtime options from the build env");
    let mut config = omnia::wasmtime::Config::from(&options);
    let triple = std::env::var("TARGET").expect("cargo env");
    config.target(&triple).unwrap_or_else(|err| {
        panic!("wasmtime cannot compile for target {triple}: {err}");
    });

    let engine = omnia::wasmtime::Engine::new(&config).expect("wasmtime engine for AOT compile");
    let component = omnia::wasmtime::component::Component::from_file(&engine, raw)
        .unwrap_or_else(|err| panic!("compiling engine component {}: {err}", raw.display()));
    let serialized = component.serialize().expect("serializing the compiled engine component");

    std::fs::write(out, serialized)
        .unwrap_or_else(|err| panic!("writing {}: {err}", out.display()));
}

// Compile the engine guest cdylib for `wasm32-wasip2` with a child
// Cargo invocation and return the built component path.
fn build_engine() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    // Re-embed whenever the engine's sources change.
    for tracked in ["src", "crates", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", manifest_dir.join(tracked).display());
    }

    check_wasm_target();

    // The exact Cargo executable driving this build, so the child
    // stays on the same toolchain without consulting rustup.
    let cargo = std::env::var_os("CARGO").expect("cargo env");
    // An isolated target dir under OUT_DIR avoids the parent build's
    // target lock; set explicitly — merely unsetting CARGO_TARGET_DIR
    // still deadlocks under a user-level `build.target-dir`.
    let target_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env")).join("engine");
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    let mut child = Command::new(cargo);
    child.current_dir(&manifest_dir).args(["build", "--lib", "--target", WASM_TARGET]);
    if release {
        child.arg("--release");
    }
    // Hold the install command's `--locked` promise through the
    // recursion (the lockfile is committed, so this is always true in
    // a checkout or an unpacked `cargo install` source).
    if manifest_dir.join("Cargo.lock").is_file() {
        child.arg("--locked");
    }
    // Sanitize before pinning the child's target directory: an ambient
    // CARGO_TARGET_DIR would otherwise be removed *after* the explicit
    // `env` call, clobbering it.
    sanitize(&mut child);
    child.env("CARGO_TARGET_DIR", &target_dir);

    let status = child
        .status()
        .unwrap_or_else(|err| panic!("failed to spawn the wasm32 engine build: {err}"));
    assert!(status.success(), "{TARGET_HINT}");

    target_dir.join(WASM_TARGET).join(if release { "release" } else { "debug" }).join("emery.wasm")
}

// Strip the parent build's Cargo/rustc env so host flags do not leak
// into the wasm32 child; `CARGO_HOME`, `CARGO_NET_OFFLINE`, and
// `RUSTUP_TOOLCHAIN` survive (the last avoids mixed compilers, E0514).
fn sanitize(child: &mut Command) {
    for (key, _) in std::env::vars_os() {
        let Some(key) = key.to_str() else { continue };
        let kept = key == "CARGO_HOME" || key == "CARGO_NET_OFFLINE";
        if key.starts_with("CARGO_") && !kept {
            child.env_remove(key);
        }
    }
    for key in ["RUSTFLAGS", "RUSTDOCFLAGS", "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        child.env_remove(key);
    }
}

// Fail fast with the install instruction when the `wasm32-wasip2`
// stdlib is missing for the active toolchain. A probe failure is not
// fatal — the child build carries the same instruction on failure.
fn check_wasm_target() {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let Ok(output) =
        Command::new(rustc).args(["--print", "target-libdir", "--target", WASM_TARGET]).output()
    else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let libdir = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    assert!(libdir.is_dir(), "{TARGET_HINT}");
}
