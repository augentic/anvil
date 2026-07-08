//! Embed-time markdown corpora: inline the judgment-prose corpus into
//! `OUT_DIR/prose/` (link-checked — a dangling relative reference
//! fails the build instead of surfacing as a lint finding) and the
//! codex packs into `OUT_DIR/codex_packs.rs` for init-time
//! materialization:
//!
//! - `codex/rules/universal/` — cross-target `UNI-*` rules authored in
//!   the sibling `augentic/specify-adapters` checkout (`SPECIFY_ADAPTERS`
//!   override).
//! - `codex/rules/core/` — framework `CORE-*` rules authored in this
//!   repo.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// The corpus [`crate::judgment::prose`] embeds: `(embedded name,
/// manifest-dir-relative source path)`.
const CORPUS: &[(&str, &str)] = &[
    ("propose.md", "src/judgment/prompts/propose.md"),
    ("synthesize.md", "src/judgment/prompts/synthesize.md"),
    ("substeps.md", "../../plugins/spec/references/synthesis/substeps.md"),
    ("requirement-block.md", "../../plugins/spec/references/synthesis/requirement-block.md"),
    ("authority.md", "../../plugins/spec/references/synthesis/authority.md"),
    ("claim-reconciliation.md", "../../plugins/spec/references/synthesis/claim-reconciliation.md"),
    ("tags.md", "../../plugins/spec/references/synthesis/tags.md"),
    ("spec-format.md", "../../plugins/spec/references/spec-format.md"),
];

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let out_root = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    let out_dir = out_root.join("prose");
    fs::create_dir_all(&out_dir).expect("create OUT_DIR/prose");

    for (name, relative) in CORPUS {
        let source = manifest_dir.join(relative);
        println!("cargo:rerun-if-changed={}", source.display());
        let body = fs::read_to_string(&source)
            .unwrap_or_else(|err| panic!("read prose corpus file {relative}: {err}"));
        check_links(&source, &body);
        fs::write(out_dir.join(name), body)
            .unwrap_or_else(|err| panic!("write embedded prose {name}: {err}"));
    }

    embed_codex(&manifest_dir, &out_root);
}

/// Resolve the adapters checkout that owns `codex/rules/universal/`.
fn adapters_checkout(manifest_dir: &Path) -> PathBuf {
    if let Ok(path) = env::var("SPECIFY_ADAPTERS") {
        return PathBuf::from(path);
    }
    for candidate in [
        manifest_dir.join("../../../specify-adapters"),
        manifest_dir.join("../../specify-adapters"),
    ] {
        if candidate.join("codex/rules/universal").is_dir() {
            return candidate;
        }
    }
    manifest_dir.join("../../../specify-adapters")
}

/// Generate `OUT_DIR/codex_packs.rs`: a sorted static slice expression
/// of `(cache-relative path, include_str!)` entries covering every
/// `.md` file under the cross-target universal pack (adapters repo)
/// and the framework `core/` pack (this repo).
fn embed_codex(manifest_dir: &Path, out_root: &Path) {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();

    let universal_dir = adapters_checkout(manifest_dir).join("codex/rules/universal");
    if !universal_dir.is_dir() {
        panic!(
            "missing cross-target codex pack at {}: clone \
             github.com/augentic/specify-adapters as a sibling of this repo \
             or set SPECIFY_ADAPTERS to that checkout",
            universal_dir.display()
        );
    }
    println!("cargo:rerun-if-changed={}", universal_dir.display());
    collect_markdown(&universal_dir, "codex/rules/universal", &mut entries);

    let core_dir = manifest_dir.join("../../codex/rules/core");
    println!("cargo:rerun-if-changed={}", core_dir.display());
    collect_markdown(&core_dir, "codex/rules/core", &mut entries);

    entries.sort();

    let mut table = String::from("&[\n");
    for (rel, path) in &entries {
        println!("cargo:rerun-if-changed={}", path.display());
        let absolute = path
            .canonicalize()
            .unwrap_or_else(|err| panic!("canonicalize codex file {}: {err}", path.display()));
        let literal = absolute.to_str().expect("codex file path is UTF-8");
        writeln!(table, "    ({rel:?}, include_str!({literal:?})),")
            .expect("write to in-memory codex table");
    }
    table.push_str("]\n");
    fs::write(out_root.join("codex_packs.rs"), table).expect("write OUT_DIR/codex_packs.rs");
}

/// Recursively collect every `.md` file under `dir` as
/// `(prefix-relative path with `/` separators, absolute path)`.
fn collect_markdown(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read codex pack directory {}: {err}", dir.display()));
    for entry in entries {
        let entry = entry.expect("read codex pack directory entry");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_str().expect("codex file name is UTF-8");
        if path.is_dir() {
            collect_markdown(&path, &format!("{prefix}/{name}"), out);
        } else if name.to_ascii_lowercase().ends_with(".md") {
            out.push((format!("{prefix}/{name}"), path));
        }
    }
}

/// Fail the build when a relative markdown link in `body` does not
/// resolve to a file on disk. Scheme-carrying URLs and same-document
/// anchors are out of scope for the embed check.
fn check_links(source: &Path, body: &str) {
    let dir = source.parent().expect("corpus file has a parent directory");
    for target in link_targets(body) {
        let path = target.split('#').next().unwrap_or_default();
        if path.is_empty() || target.contains("://") || target.starts_with("mailto:") {
            continue;
        }
        let resolved = dir.join(path);
        assert!(
            resolved.exists(),
            "dangling reference in {}: `{target}` does not resolve (checked {})",
            source.display(),
            resolved.display()
        );
    }
}

/// Every inline markdown link destination (`](…)`) in `body`, in order.
fn link_targets(body: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find("](") {
        rest = &rest[open + 2..];
        let Some(close) = rest.find(')') else {
            break;
        };
        targets.push(rest[..close].trim());
        rest = &rest[close + 1..];
    }
    targets
}
