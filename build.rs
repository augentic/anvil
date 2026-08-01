//! Builds, ahead-of-time compiles, and embeds the wasm32 engine
//! component the shipped binary boots.
//!
//! A child `cargo build --lib --target wasm32-wasip2` into an
//! isolated target directory under `OUT_DIR` produces the raw engine
//! component (so plain `cargo install --git … --locked` produces a
//! bootable binary), which is then serialized to a native wasmtime
//! artifact at `$OUT_DIR/emery.bin` for the `include_bytes!` in
//! `src/main.rs` — startup deserializes instead of JIT-compiling the
//! engine. There is no placeholder fallback: a native binary either
//! embeds a real engine or fails to build with a direct instruction.

use std::path::PathBuf;
use std::process::Command;

/// The engine guest's compilation target.
const WASM_TARGET: &str = "wasm32-wasip2";

/// Install instruction surfaced whenever the child build cannot run.
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

    precompile(&engine);
}

/// Ahead-of-time compile the raw engine component into the serialized
/// wasmtime artifact at `$OUT_DIR/emery.bin` that `src/main.rs`
/// embeds.
///
/// The engine configuration mirrors the runtime loader: the same
/// `RuntimeOptions` env-driven compile-affecting settings (`MAX_FUEL`,
/// `BRANCH_HINTING`, `MEMORY_RESERVATION`, `MEMORY_GUARD_SIZE`,
/// `DEBUG_SYMBOLS`, `GENERATE_ADDRESS_MAP`) must match between this
/// build and the running binary or deserialization rejects the
/// artifact at startup. Cargo's `TARGET` pins the code to the
/// binary's triple, so cross-compiled binaries embed a loadable
/// artifact rather than one for the build host.
fn precompile(raw: &std::path::Path) {
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

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env")).join("emery.bin");
    std::fs::write(&out, serialized)
        .unwrap_or_else(|err| panic!("writing {}: {err}", out.display()));
}

/// Compile the engine guest cdylib for `wasm32-wasip2` with a child
/// Cargo invocation and return the built component path.
fn build_engine() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    // Re-embed whenever the engine's sources change.
    for tracked in ["src/lib.rs", "crates", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", manifest_dir.join(tracked).display());
    }

    check_wasm_target();

    // The exact Cargo executable driving this build, so the child
    // stays on the same toolchain without consulting rustup.
    let cargo = std::env::var_os("CARGO").expect("cargo env");
    // An isolated target directory under OUT_DIR avoids the parent
    // build's target-directory lock. Set explicitly — merely unsetting
    // CARGO_TARGET_DIR still deadlocks for users with
    // `build.target-dir` in their Cargo configuration.
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

/// Strip the parent build's Cargo and rustc environment from the
/// child so host flags (`RUSTFLAGS=-Dwarnings`, wrappers) do not leak
/// into the wasm32 build. `CARGO_HOME` and `CARGO_NET_OFFLINE`
/// survive so registry caches and offline policy carry over.
/// `RUSTUP_TOOLCHAIN` is deliberately kept: it pins every rustc the
/// child spawns to the parent's toolchain — removing it lets the
/// rustup shim fall back to the machine default mid-build, mixing
/// compiler versions (E0514).
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

/// Fail fast with the install instruction when the `wasm32-wasip2`
/// standard library is not installed for the active toolchain. A
/// probe failure is not fatal — the child build carries the same
/// instruction on failure.
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
