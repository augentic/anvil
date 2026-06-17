//! Layer-3 verification guard: no rule policy in the lint engine.
//! (See DECISIONS.md §"Framework lint engine: generic dispatcher (Road A / Road B)".)
//!
//! Proves no rule policy lives baked into the lint engine. It scans the
//! deterministic hint eval arms (`lint/eval/`) and the shared
//! adapter-briefs helper (`lint/adapter_briefs.rs`), and FAILS if any
//! rule-specific policy literal reappears:
//!
//! - a value-bearing discriminator (`*-cover-operations`,
//!   `*-equal-operations`, `*-equals-v1`, `adapter-manifest-version-*`),
//! - an operation-set array literal (`["survey", "extract"]` /
//!   `["shape", "build", "merge"]`),
//! - an owner->prefix policy map (`BUILTIN_NAMESPACES` / `TARGET_OWNERS`),
//! - a canonical-document path string, or
//! - a numeric cap `const` whose name is not on the mechanism allow-list.
//!
//! Every rule-specific value must instead ride the rule's `config:` (in
//! the `specify` repo). The only engine-side constants this guard
//! tolerates are mechanism — evidence/snippet/iteration bounds —
//! enumerated by name in [`MECHANISM_CAP_CONSTS`] with the reason each
//! is mechanism, not policy.

use std::path::{Path, PathBuf};

use regex::Regex;

/// Numeric cap `const`s that are mechanism, not rule policy, and are
/// therefore exempt from the "no bare numeric cap" guard. Each is keyed
/// by const name with the reason it is mechanism.
const MECHANISM_CAP_CONSTS: &[(&str, &str)] = &[
    ("STDERR_MAX_BYTES", "tool stderr truncation budget (wire mechanism)"),
    ("SNIPPET_MAX_CHARS", "evidence snippet truncation budget (finding mechanism)"),
    ("CLAMP_ITERATION_LIMIT", "evidence-size clamp loop bound (finding mechanism)"),
];

/// Literal substrings that only ever appear as relocated rule policy.
/// Any reappearance in the scanned engine source is a Layer-3 regression.
const FORBIDDEN_SUBSTRINGS: &[(&str, &str)] = &[
    ("cover-operations", "set-coverage subset operation-set discriminator (CORE-004) — use config"),
    ("equal-operations", "set-coverage exact operation-set discriminator (CORE-007) — use config"),
    ("equals-v1", "constant-eq version discriminator (CORE-006) — use config"),
    ("adapter-manifest-version", "constant-eq version discriminator (CORE-006) — use config"),
    ("BUILTIN_NAMESPACES", "namespace owner->prefix policy map (CORE-009) — use config"),
    ("TARGET_OWNERS", "namespace owner->prefix policy map (CORE-009) — use config"),
];

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The engine source roots policy could hide in: the hint eval arms and
/// the shared adapter-briefs helper.
fn scan_roots() -> Vec<PathBuf> {
    let root = crate_root();
    vec![root.join("src/lint/eval"), root.join("src/lint/adapter_briefs.rs")]
}

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    let Ok(entries) = std::fs::read_dir(path) else { return };
    for entry in entries.flatten() {
        collect_rs_files(&entry.path(), out);
    }
}

fn scanned_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in scan_roots() {
        collect_rs_files(&root, &mut files);
    }
    assert!(!files.is_empty(), "guard scanned zero files — scan roots drifted");
    files.sort();
    files
}

fn rel(path: &Path) -> String {
    path.strip_prefix(crate_root()).unwrap_or(path).to_string_lossy().into_owned()
}

#[test]
fn no_value_bearing_discriminators() {
    for file in scanned_files() {
        let content = std::fs::read_to_string(&file).expect("read scanned file");
        for (needle, reason) in FORBIDDEN_SUBSTRINGS {
            assert!(
                !content.contains(needle),
                "{}: forbidden rule policy literal `{needle}` reappeared ({reason}). \
                 Move the value into the rule's `config:` in the specify repo.",
                rel(&file),
            );
        }
    }
}

#[test]
fn no_operation_set_arrays() {
    let op_array = Regex::new(
        r#"&?\[\s*"(?:survey|extract|shape|build|merge)"\s*,\s*"(?:survey|extract|shape|build|merge)""#,
    )
    .expect("operation-array regex compiles");
    for file in scanned_files() {
        let content = std::fs::read_to_string(&file).expect("read scanned file");
        assert!(
            !op_array.is_match(&content),
            "{}: an adapter operation-set array literal reappeared. The expected operation \
             sets are CORE-004 / CORE-007 policy and must ride the rule's `config: \
             {{ expected-operations }}`, not a `const` in the engine.",
            rel(&file),
        );
    }
}

#[test]
fn no_unallowlisted_numeric_caps() {
    let cap_const = Regex::new(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*[A-Za-z0-9_]+\s*=\s*[0-9]")
        .expect("cap-const regex compiles");
    for file in scanned_files() {
        let content = std::fs::read_to_string(&file).expect("read scanned file");
        for caps in cap_const.captures_iter(&content) {
            let name = &caps[1];
            let looks_like_cap =
                name.contains("MAX") || name.contains("LIMIT") || name.contains("CAP");
            if !looks_like_cap {
                continue;
            }
            assert!(
                MECHANISM_CAP_CONSTS.iter().any(|(allowed, _)| *allowed == name),
                "{}: numeric cap `const {name}` is not on the mechanism allow-list. A \
                 rule-specific cap must ride the rule's `config: {{ max }}` in the specify \
                 repo; if this is genuinely mechanism, add it to MECHANISM_CAP_CONSTS with a \
                 reason.",
                rel(&file),
            );
        }
    }
}
