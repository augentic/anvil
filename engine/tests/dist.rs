//! Staleness gate for the committed, embedded workflow guest.
//!
//! The `specify` binary `include_bytes!`s the release-built component at
//! `crates/workflow-guest/guest.wasm`; `cargo make dist-guest` refreshes
//! it and records the sidecar checked here. The gate recomputes both
//! recorded digests — the component bytes and the guest-source
//! fingerprint — so a hand-edited artifact or guest-reachable source
//! edited without re-running `dist-guest` fails CI. See DECISIONS.md
//! §"Workflow-guest distribution".

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use specify_schema::cache::tree_content_digest;
use specify_schema::digest::sha256_hex;

/// Workspace crates in the workflow guest's dependency closure
/// (`cargo tree -p specify-workflow-guest --target wasm32-wasip2`).
/// Editing any of these trees can change guest behavior, so each
/// contributes to the recorded source fingerprint.
const GUEST_GRAPH_CRATES: &[&str] = &[
    "diagnostics",
    "dispatch",
    "error",
    "extension",
    "guest-model",
    "model",
    "schema",
    "workflow",
    "workflow-guest",
];

/// Repo-root trees the guest embeds beyond crate sources: the WIT
/// package `wit_bindgen::generate!` binds and the JSON Schemas
/// `specify-schema` includes.
const GUEST_GRAPH_TREES: &[&str] = &["wit", "schemas"];

fn engine_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn guest_wasm_path() -> PathBuf {
    engine_root().join("crates/workflow-guest/guest.wasm")
}

fn sidecar_path() -> PathBuf {
    engine_root().join("crates/workflow-guest/guest.wasm.sha256")
}

/// SHA-256 hex of the committed component bytes.
fn wasm_digest() -> String {
    let bytes = fs::read(guest_wasm_path()).expect("read committed guest.wasm");
    sha256_hex(&bytes)
}

/// Deterministic fingerprint over every guest-reachable source tree:
/// the `(label, tree-digest)` lines of each closure crate's `src/` +
/// `Cargo.toml` and the repo-root embedded trees, folded into one
/// SHA-256. Known boundary (documented in DECISIONS.md): external
/// dependency bumps and toolchain changes are not fingerprinted.
fn sources_digest() -> String {
    let engine = engine_root();
    let repo = engine.parent().expect("engine has a parent repo root").to_path_buf();
    let mut ledger = String::new();
    for name in GUEST_GRAPH_CRATES {
        let crate_dir = engine.join("crates").join(name);
        record(&mut ledger, &format!("crates/{name}/src"), &crate_dir.join("src"));
        let manifest = fs::read(crate_dir.join("Cargo.toml")).expect("read crate Cargo.toml");
        writeln!(ledger, "crates/{name}/Cargo.toml sha256:{}", sha256_hex(&manifest))
            .expect("write ledger line");
    }
    for tree in GUEST_GRAPH_TREES {
        record(&mut ledger, tree, &repo.join(tree));
    }
    sha256_hex(ledger.as_bytes())
}

fn record(ledger: &mut String, label: &str, tree: &Path) {
    assert!(tree.is_dir(), "guest-graph tree missing: {}", tree.display());
    writeln!(ledger, "{label} {}", tree_content_digest(tree)).expect("write ledger line");
}

fn render_sidecar(wasm: &str, sources: &str) -> String {
    format!("wasm: {wasm}\nsources: {sources}\n")
}

// The committed sidecar must match both recomputed digests: `wasm:`
// catches a drifted/hand-edited component, `sources:` catches
// guest-reachable source edited without re-running `cargo make
// dist-guest`. Regenerate only via `dist-guest` (it rebuilds the
// component before re-recording, so the sidecar can never pair a fresh
// source fingerprint with a stale component).
#[test]
fn embedded_guest_sidecar() {
    let actual = render_sidecar(&wasm_digest(), &sources_digest());

    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        fs::write(sidecar_path(), actual).expect("write guest.wasm.sha256");
        return;
    }

    let recorded = fs::read_to_string(sidecar_path()).unwrap_or_else(|err| {
        panic!(
            "sidecar {} missing ({err}); run `cargo make dist-guest` and commit the result",
            sidecar_path().display()
        )
    });
    assert_eq!(
        actual, recorded,
        "committed workflow guest is stale against its sidecar: run `cargo make dist-guest` and \
         commit the refreshed crates/workflow-guest/guest.wasm + guest.wasm.sha256"
    );
}
