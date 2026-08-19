//! Builds the wasm32 engine component with a child cargo build and
//! embeds it at `$OUT_DIR/emery.bin` (AOT-serialized in release, raw
//! in debug), and generates the embedded first-party registry.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

/// The engine guest's compilation target.
const WASM_TARGET: &str = "wasm32-wasip2";

/// Install instruction surfaced whenever the child build cannot run.
const TARGET_HINT: &str = "the wasm32 engine could not be built; install the target with `rustup \
                           target add wasm32-wasip2` and retry";

fn main() {
    // The wasm32 build runs this script too (the engine guest cdylib
    // embeds nothing; the `launcher` module is cfg'd out); returning
    // here is the recursion guard for the child build below.
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    embed_registry();

    let engine = build_engine();

    let len = std::fs::metadata(&engine).map(|meta| meta.len()).unwrap_or_default();
    assert!(len > 0, "engine component at {} is empty; refusing to embed it", engine.display());

    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo env")).join("emery.bin");
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

/// Generate the embedded first-party registry (ADR-0002 §2) at
/// `$OUT_DIR/embedded.rs` (included by `src/launcher.rs`)
/// from `EMERY_EMBED_DIR` — built `<name>.wasm` components staged by
/// the release build (first-party adapters) and by the journey rung
/// (the mock component). Unset, missing, or empty, the table is empty
/// and resolution stays local (cache seed, store).
fn embed_registry() {
    println!("cargo:rerun-if-env-changed=EMERY_EMBED_DIR");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let mut rows = String::new();
    for (name, path) in components() {
        #[expect(
            clippy::unnecessary_debug_formatting,
            reason = "Debug formatting emits the quoted, escaped path literal include_bytes! needs"
        )]
        writeln!(rows, "    (\"{name}\", include_bytes!({path:?}).as_slice()),")
            .expect("write to a String");
    }
    let table = format!(
        "/// First-party components embedded in the binary as default\n\
         /// registry entries (ADR-0002 \u{a7}2), staged from `EMERY_EMBED_DIR`\n\
         /// at build time.\n\
         const EMBEDDED: &[(&str, &[u8])] = &[\n{rows}];\n"
    );
    std::fs::write(out.join("embedded.rs"), table).expect("write the embedded registry table");
}

/// Every `<name>.wasm` under `EMERY_EMBED_DIR`, sorted by the adapter
/// name its file stem derives (the `emery_` artifact prefix stripped,
/// underscores folded to kebab dashes — mirroring
/// `emery_engine::resolve::name_from_component`).
fn components() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("EMERY_EMBED_DIR") else {
        return Vec::new();
    };
    let dir = PathBuf::from(dir);
    println!("cargo:rerun-if-changed={}", dir.display());
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut components: Vec<(String, PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            // Cargo also writes `{name}-{hash}.wasm` beside the
            // example artifact; those must not become registry keys.
            if fingerprint_stem(stem) {
                return None;
            }
            println!("cargo:rerun-if-changed={}", path.display());
            let name = stem
                .strip_prefix("emery_")
                .or_else(|| stem.strip_prefix("emery-"))
                .unwrap_or(stem)
                .replace('_', "-");
            Some((name, path))
        })
        .collect();
    components.sort();
    components
}

/// Cargo's extra `{name}-{16 hex}.wasm` copy beside an example artifact.
fn fingerprint_stem(stem: &str) -> bool {
    stem.rsplit_once('-').is_some_and(|(_, suffix)| {
        suffix.len() == 16 && suffix.bytes().all(|b| b.is_ascii_hexdigit())
    })
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
    for tracked in ["src/lib.rs", "crates", "Cargo.toml", "Cargo.lock"] {
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
