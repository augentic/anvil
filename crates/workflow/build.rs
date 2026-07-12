//! Embed-time markdown corpus: inline the judgment-prose corpus into
//! `OUT_DIR/prose/` (link-checked — a dangling relative reference
//! fails the build instead of surfacing at runtime).

use std::path::{Path, PathBuf};
use std::{env, fs};

/// The corpus [`crate::judgment::prose`] embeds: `(embedded name,
/// manifest-dir-relative source path)`.
const CORPUS: &[(&str, &str)] = &[
    ("propose.md", "prompts/propose.md"),
    ("synthesize.md", "prompts/synthesize.md"),
    ("substeps.md", "prompts/synthesis/substeps.md"),
    ("requirement-block.md", "prompts/synthesis/requirement-block.md"),
    ("authority.md", "prompts/synthesis/authority.md"),
    ("claim-reconciliation.md", "prompts/synthesis/claim-reconciliation.md"),
    ("tags.md", "prompts/synthesis/tags.md"),
    ("decisions.md", "prompts/synthesis/decisions.md"),
    ("spec-format.md", "prompts/synthesis/spec-format.md"),
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
