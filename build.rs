//! Builds the wasm32 engine with a child cargo build, embeds it at
//! `$OUT_DIR/emery.cwasm` (AOT-serialized in release, raw in debug),
//! and stages first-party components as static guests.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

/// The engine guest's compilation target.
const WASM_TARGET: &str = "wasm32-wasip2";

/// Install instruction surfaced whenever the child build cannot run.
const TARGET_HINT: &str = "the wasm32 engine could not be built; install the target with `rustup \
                           target add wasm32-wasip2` and retry";

fn main() {
    println!("cargo::rustc-check-cfg=cfg(emery_first_party)");
    // The wasm32 build runs this script too (the engine guest cdylib
    // embeds nothing); returning here is the recursion guard for the
    // child build below.
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    stage_first_party();

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

/// Stage the pinned first-party components (`scripts/first-party.txt`)
/// as static guests: generate `$OUT_DIR/first_party.rs` — one bytes
/// constant and one pinned-id constant per adapter, consumed by
/// `src/main.rs` — and set the `emery_first_party` cfg.
///
/// `EMERY_EMBED_DIR` unset builds the engine-only binary (kernel and
/// wire suites need no adapters). Set, every pin must be staged as
/// `<dir>/<name>.wasm` — a partial set fails the build rather than
/// shipping a binary missing a documented adapter.
fn stage_first_party() {
    println!("cargo:rerun-if-env-changed=EMERY_EMBED_DIR");
    println!("cargo:rerun-if-changed=scripts/first-party.txt");
    let Some(dir) = std::env::var_os("EMERY_EMBED_DIR") else {
        return;
    };
    let dir = PathBuf::from(dir);
    let mut consts = String::new();
    for (name, version) in first_party_pins() {
        let path = dir.join(format!("{name}.wasm"));
        assert!(
            path.is_file(),
            "EMERY_EMBED_DIR is set but `{name}` is not staged at {}; stage every pin in \
             scripts/first-party.txt or unset EMERY_EMBED_DIR for an engine-only build",
            path.display()
        );
        println!("cargo:rerun-if-changed={}", path.display());
        let upper = name.to_uppercase().replace('-', "_");
        #[expect(
            clippy::unnecessary_debug_formatting,
            reason = "Debug formatting emits the quoted, escaped path literal include_bytes! needs"
        )]
        write!(
            consts,
            "/// Staged `{name}` component bytes.\n\
             pub const {upper}: &[u8] = include_bytes!({path:?});\n\
             /// The pinned `{name}` routed id.\n\
             pub const {upper}_PIN: &str = \"source:{name}@{version}\";\n"
        )
        .expect("write to a String");
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("first_party.rs"), consts).expect("write the first-party constants");
    println!("cargo:rustc-cfg=emery_first_party");
}

/// The `<name> <version>` pins in `scripts/first-party.txt`, comments
/// and blank lines skipped.
fn first_party_pins() -> Vec<(String, String)> {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo env"));
    let pins = std::fs::read_to_string(manifest_dir.join("scripts/first-party.txt"))
        .expect("read scripts/first-party.txt");
    pins.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, version) = line
                .split_once(' ')
                .unwrap_or_else(|| panic!("malformed first-party pin `{line}`"));
            (name.to_owned(), version.trim().to_owned())
        })
        .collect()
}

/// Ahead-of-time compile the raw engine component into the serialized
/// wasmtime artifact at `out` that `src/main.rs` embeds.
///
/// The env-driven compile-affecting `RuntimeOptions` must match
/// between this build and the running binary or deserialization
/// rejects the artifact at startup; cargo's `TARGET` pins the code to
/// the binary's triple, so cross-compiles embed a loadable artifact.
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

/// Compile the engine guest cdylib for `wasm32-wasip2` with a child
/// Cargo invocation and return the built component path.
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
