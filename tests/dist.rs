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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn guest_wasm_path() -> PathBuf {
    workspace_root().join("crates/workflow-guest/guest.wasm")
}

fn sidecar_path() -> PathBuf {
    workspace_root().join("crates/workflow-guest/guest.wasm.sha256")
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
    let root = workspace_root();
    let mut ledger = String::new();
    for name in GUEST_GRAPH_CRATES {
        let crate_dir = root.join("crates").join(name);
        record(&mut ledger, &format!("crates/{name}/src"), &crate_dir.join("src"));
        let manifest = fs::read(crate_dir.join("Cargo.toml")).expect("read crate Cargo.toml");
        writeln!(ledger, "crates/{name}/Cargo.toml sha256:{}", sha256_hex(&manifest))
            .expect("write ledger line");
    }
    for tree in GUEST_GRAPH_TREES {
        record(&mut ledger, tree, &root.join(tree));
    }
    sha256_hex(ledger.as_bytes())
}

fn record(ledger: &mut String, label: &str, tree: &Path) {
    assert!(tree.is_dir(), "guest-graph tree missing: {}", tree.display());
    writeln!(ledger, "{label} {}", tree_content_digest(tree)).expect("write ledger line");
}

/// Deterministic `sha256:<hex>` digest over a source tree: files
/// name-sorted by slash-relative path, each contributing its path,
/// a NUL separator, its length, and its bytes. (Local to this gate —
/// the shared helper retired with the tree-packed store in RFC-64,
/// where a store entry became a single component file.)
fn tree_content_digest(entry: &Path) -> String {
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect_entry_files(entry, entry, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = specify_schema::digest::Hasher::new();
    for (rel, path) in &files {
        let bytes = fs::read(path).unwrap_or_default();
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    format!("sha256:{}", hasher.finalize_hex())
}

fn collect_entry_files(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            collect_entry_files(root, &path, out);
        } else if meta.is_file()
            && let Some(rel) = relative_slash_path(root, &path)
        {
            out.push((rel, path));
        }
    }
}

fn relative_slash_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let mut parts = Vec::new();
    for component in rel.components() {
        parts.push(component.as_os_str().to_str()?);
    }
    Some(parts.join("/"))
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
