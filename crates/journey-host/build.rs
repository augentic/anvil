//! Builds the same wasm32 engine guest the shipped binary embeds
//! (ADR-0009 §5) with a child cargo build, embedded raw at
//! `$OUT_DIR/emery.bin` (JIT at startup; no dev-only AOT pass).

use std::path::PathBuf;
use std::process::Command;

/// The engine guest's compilation target.
const WASM_TARGET: &str = "wasm32-wasip2";

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    let workspace = manifest_dir.parent().and_then(std::path::Path::parent).expect("repo root");
    for tracked in ["src", "crates", "wit", "Cargo.toml", "Cargo.lock"] {
        println!("cargo:rerun-if-changed={}", workspace.join(tracked).display());
    }

    // The exact Cargo executable driving this build, so the child
    // stays on the same toolchain without consulting rustup.
    let cargo = std::env::var_os("CARGO").expect("cargo env");
    // An isolated target dir under OUT_DIR avoids the parent build's
    // target lock (a user-level `build.target-dir` would deadlock).
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env"));
    let target_dir = out_dir.join("engine");

    let mut child = Command::new(cargo);
    child.current_dir(workspace).args([
        "build",
        "--locked",
        "--lib",
        "-p",
        "emery",
        "--target",
        WASM_TARGET,
    ]);
    sanitize(&mut child);
    child.env("CARGO_TARGET_DIR", &target_dir);

    let status =
        child.status().unwrap_or_else(|err| panic!("spawning the wasm32 engine build: {err}"));
    assert!(
        status.success(),
        "the wasm32 engine could not be built; install the target with `rustup target add \
         wasm32-wasip2` and retry"
    );

    let engine = target_dir.join(WASM_TARGET).join("debug").join("emery.wasm");
    let len = std::fs::metadata(&engine).map(|meta| meta.len()).unwrap_or_default();
    assert!(len > 0, "engine component at {} is empty; refusing to embed it", engine.display());
    let out = out_dir.join("emery.bin");
    std::fs::copy(&engine, &out)
        .unwrap_or_else(|err| panic!("copying {} to {}: {err}", engine.display(), out.display()));
}

/// Strip the parent build's Cargo and rustc environment from the
/// child so host flags (`RUSTFLAGS=-Dwarnings`, wrappers) do not leak
/// into the wasm32 build; `CARGO_HOME` / `CARGO_NET_OFFLINE` and
/// `RUSTUP_TOOLCHAIN` survive (registry caches, offline policy, and a
/// pinned toolchain must carry over — the root `build.rs` records the
/// E0514 lesson).
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
