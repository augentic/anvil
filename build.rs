//! Builds and embeds the wasm32 engine component.

// Build scripts are host-side, and Cargo's build protocol requires environment access.
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "host-side build script; the wasm32 guest deny-list does not apply"
)]

use std::path::PathBuf;
use std::process::Command;

const WASM_TARGET: &str = "wasm32-wasip2";

const TARGET_HINT: &str = "the wasm32 engine could not be built; install the target with `rustup \
                           target add wasm32-wasip2` and retry";

fn main() {
    // The child wasm build reruns this script; return to prevent recursion.
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
        // Debug uses JIT to avoid repeated Cranelift AOT cost during edits and CI.
        std::fs::copy(&engine, &out).unwrap_or_else(|err| {
            panic!("copying {} to {}: {err}", engine.display(), out.display())
        });
    }
}

// Runtime options and Cargo's target triple must match the consuming binary.
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

fn build_engine() -> PathBuf {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    for tracked in ["src", "crates", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", manifest_dir.join(tracked).display());
    }

    check_wasm_target();

    // Reuse Cargo from the parent build to stay on its toolchain.
    let cargo = std::env::var_os("CARGO").expect("cargo env");
    // Isolation avoids the parent's target lock; unsetting CARGO_TARGET_DIR
    // alone still deadlocks when the user configures build.target-dir.
    let target_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env")).join("engine");
    let release = std::env::var("PROFILE").as_deref() == Ok("release");

    let mut child = Command::new(cargo);
    child.current_dir(&manifest_dir).args(["build", "--lib", "--target", WASM_TARGET]);
    if release {
        child.arg("--release");
    }
    // Preserve the parent install's locked dependency guarantee.
    if manifest_dir.join("Cargo.lock").is_file() {
        child.arg("--locked");
    }
    // Sanitize first so ambient CARGO_TARGET_DIR removal cannot clobber this value.
    sanitize(&mut child);
    child.env("CARGO_TARGET_DIR", &target_dir);

    let status = child
        .status()
        .unwrap_or_else(|err| panic!("failed to spawn the wasm32 engine build: {err}"));
    assert!(status.success(), "{TARGET_HINT}");

    target_dir.join(WASM_TARGET).join(if release { "release" } else { "debug" }).join("emery.wasm")
}

// Prevent host flags from leaking into wasm; preserve Cargo settings and the
// inherited toolchain to avoid mixed-compiler E0514 failures.
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

// A successful probe fails fast on a missing stdlib; probe failures defer to
// the child build, which reports the same installation hint.
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
