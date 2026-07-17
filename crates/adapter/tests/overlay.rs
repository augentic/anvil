//! The dev-only prose overlay (`SPECIFY_PROSE_OVERLAY=1`): overlay
//! bodies win, misses fall back to the embedded table, the doc set
//! never changes, and without the env grant the probe is inert.

use std::fs;

use adapter::registry::{Doc, body, find, resolve};
use tempfile::TempDir;

/// A sorted table, as the `prose` codegen emits.
static DOCS: &[Doc] = &[
    Doc {
        path: "prompts/build.md",
        body: "# embedded build",
    },
    Doc {
        path: "references/verifier.md",
        body: "# embedded verifier",
    },
];

// Rebase the process cwd into a fresh tempdir seeded with `.eval/prose/`
// overlay files and grant the overlay env var — the overlay resolves
// against the cwd. Mutating the cwd and env is safe here because
// `cargo make test` runs under nextest with process-per-test isolation.
// The returned guard keeps the tree alive.
fn enter_overlay(files: &[(&str, &str)]) -> TempDir {
    let dir = seed_tree(files);
    grant_overlay();
    dir
}

// Seed the tree without the env grant: the probe must stay inert.
fn seed_tree(files: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    for (path, contents) in files {
        let file = dir.path().join(".eval/prose").join(path);
        let parent = file.parent().expect("overlay files sit under .eval/prose");
        fs::create_dir_all(parent).expect("create overlay tree");
        fs::write(&file, contents).expect("write overlay file");
    }
    std::env::set_current_dir(dir.path()).expect("enter tempdir");
    dir
}

// Same cwd rebase, but with a directory squatting on the overlay path —
// present but unreadable as a file.
fn enter_overlay_with_dir_at(path: &str) -> TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    fs::create_dir_all(dir.path().join(".eval/prose").join(path)).expect("create overlay dir");
    std::env::set_current_dir(dir.path()).expect("enter tempdir");
    grant_overlay();
    dir
}

#[expect(
    unsafe_code,
    reason = "edition-2024 `set_var` is unsafe; nextest runs each test in its own \
              process, so no other thread touches the environment concurrently"
)]
fn grant_overlay() {
    // SAFETY: process-per-test isolation (see the lint expectation above).
    unsafe { std::env::set_var("SPECIFY_PROSE_OVERLAY", "1") };
}

#[test]
fn overlay_wins() {
    let _dir = enter_overlay(&[("prompts/build.md", "# overlaid build")]);
    assert_eq!(body(DOCS, "prompts/build.md"), "# overlaid build");
    assert_eq!(resolve(DOCS, "prompts/build.md"), Some("# overlaid build"));
}

#[test]
fn absent_serves_embedded() {
    let _dir = enter_overlay(&[("prompts/build.md", "# overlaid build")]);
    assert_eq!(body(DOCS, "references/verifier.md"), "# embedded verifier");
    assert_eq!(resolve(DOCS, "references/verifier.md"), Some("# embedded verifier"));
}

// Without the env grant a seeded overlay tree is ignored: the probe is
// inert by default, so a published component serves embedded bodies.
#[test]
fn ungranted_serves_embedded() {
    let _dir = seed_tree(&[("prompts/build.md", "# overlaid build")]);
    assert_eq!(body(DOCS, "prompts/build.md"), "# embedded build");
    assert_eq!(resolve(DOCS, "prompts/build.md"), Some("# embedded build"));
}

// An empty overlay file is served as-is by design: `read_to_string`
// reads to EOF, so a partial read cannot masquerade as an empty body.
#[test]
fn empty_serves_empty() {
    let _dir = enter_overlay(&[("prompts/build.md", "")]);
    assert_eq!(body(DOCS, "prompts/build.md"), "");
}

// A present-but-unreadable overlay path (a directory here) must fail
// loud rather than silently fall back to the embedded body.
#[test]
#[should_panic(expected = "is unreadable")]
fn unreadable_panics() {
    let _dir = enter_overlay_with_dir_at("prompts/build.md");
    let _ = body(DOCS, "prompts/build.md");
}

#[test]
#[should_panic(expected = "document `prompts/missing.md` is not embedded")]
fn miss_in_both_panics() {
    let _dir = enter_overlay(&[("prompts/build.md", "# overlaid build")]);
    let _ = body(DOCS, "prompts/missing.md");
}

// An overlay file for a path outside the embedded table never extends
// the doc set: existence is always the table's.
#[test]
fn never_adds_entries() {
    let _dir = enter_overlay(&[("prompts/extra.md", "# not in the table")]);
    assert!(find(DOCS, "prompts/extra.md").is_none());
    assert_eq!(resolve(DOCS, "prompts/extra.md"), None);
}

#[test]
#[should_panic(expected = "document `prompts/extra.md` is not embedded")]
fn overlay_only_still_panics() {
    let _dir = enter_overlay(&[("prompts/extra.md", "# not in the table")]);
    let _ = body(DOCS, "prompts/extra.md");
}
